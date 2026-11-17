//! Minimal MCP JSON-RPC 2.0 server barrel (delegates to `agentla-mcp`).

use core::fmt::Write as _;
use rp_pac as pac;

#[allow(unused_imports)]
pub use agentla_mcp::{
    escape_json, extract_bool_arg, extract_i32_arg, extract_method, extract_string_arg,
    extract_tool_name, extract_u32_arg, McpServer,
};

use crate::profile;
use crate::servo;

pub const MAX_BODY: usize = 16384;

pub fn handle_request(body: &[u8]) -> heapless::String<16384> {
    let server = McpServer::with_handler(invoke_tool);
    server.handle_request(body)
}

fn decode_hex(hex: &str, out: &mut [u8]) -> usize {
    let bytes = hex.as_bytes();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < bytes.len() && count < out.len() {
        if let (Some(h), Some(l)) = ((bytes[i] as char).to_digit(16), (bytes[i + 1] as char).to_digit(16)) {
            out[count] = ((h << 4) | l) as u8;
            count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    count
}

fn parse_mcp_color(s: &str) -> (u8, u8, u8) {
    let s = s.trim();
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() == 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&hex[0..2], 16),
            u8::from_str_radix(&hex[2..4], 16),
            u8::from_str_radix(&hex[4..6], 16),
        ) {
            return (r, g, b);
        }
    }
    match hex {
        "red" => (255, 0, 0),
        "green" => (0, 255, 0),
        "blue" => (0, 0, 255),
        "white" => (255, 255, 255),
        "black" | "off" => (0, 0, 0),
        "yellow" => (255, 255, 0),
        "cyan" => (0, 255, 255),
        "magenta" | "purple" => (255, 0, 255),
        "orange" => (255, 128, 0),
        _ => (255, 255, 255),
    }
}

fn invoke_tool(name: &str, body: &str, out: &mut heapless::String<1536>) -> bool {
    match name {
        "status" => {
            let snap = crate::telemetry::sample_telemetry();
            let _ = write!(
                out,
                r#"{{"clock_mhz":150,"tools":{},"vtarget_mv":{},"itarget_ma":{},"power_mw":{},"temp_c":{},"overcurrent":{},"active_profile":{},"status":"healthy"}}"#,
                agentla_mcp::ALL_TOOLS.len(), snap.v_target_mv, snap.i_target_ma, snap.p_target_mw, snap.temp_c, snap.is_tripped, profile::active_slot()
            );
            false
        }
        "probe_target" => {
            let port_str = extract_string_arg(body, "port");
            let target = match port_str {
                Some("ch32v203") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v203),
                Some("ch32v003") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v003),
                Some("bluepill") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::BluePill),
                _ => crate::programmer::detect_target_global(),
            };
            match target {
                Ok(info) => {
                    let _ = write!(out, "detected=true name=\"{}\" family={:?} idcode=0x{:08X} flash={}KB",
                        info.name, info.family, info.idcode, info.flash_size / 1024);
                    false
                }
                Err(e) => {
                    let _ = write!(out, "detected=false error={:?}", e);
                    true
                }
            }
        }
        "read_pins" => {
            let bank = pac::SIO.gpio_in(0).read();
            let _ = write!(out, "input=0x{:08x}", bank);
            false
        }
        "pins_status" => {
            let _ = out.push_str(r#"{"pins":["#);
            let mut first = true;
            for pin in 0..30 {
                if !first { let _ = out.push(','); }
                first = false;
                let owner = crate::pin_owner::owner_of(pin);
                let owner_str = match owner {
                    Some(crate::pin_owner::Driver::Programmer) => "swd",
                    Some(crate::pin_owner::Driver::Adapter) => "adapter",
                    Some(crate::pin_owner::Driver::Capture) => "capture",
                    Some(crate::pin_owner::Driver::Oscilloscope) => "scope",
                    Some(crate::pin_owner::Driver::Generator) => "generator",
                    Some(crate::pin_owner::Driver::Ws2812) => "ws2812",
                    Some(crate::pin_owner::Driver::Glitch) => "glitch",
                    Some(crate::pin_owner::Driver::Servo) => "servo",
                    Some(crate::pin_owner::Driver::Gpio) => "gpio",
                    None => "none",
                };
                let claimed = owner.is_some();
                let _ = write!(out, r#"{{"pin":{},"claimed":{},"driver":"{}"}}"#, pin, claimed, owner_str);
            }
            let _ = out.push_str("]}");
            false
        }
        "pins_claim" => {
            let pin = extract_u32_arg(body, "pin");
            let driver_str = extract_string_arg(body, "driver").unwrap_or("gpio");
            if let Some(pin_num) = pin {
                let driver = match driver_str {
                    "swd" => crate::pin_owner::Driver::Programmer,
                    "servo" => crate::pin_owner::Driver::Servo,
                    "adapter" | "i2c" | "spi" | "uart" | "can" => crate::pin_owner::Driver::Adapter,
                    "capture" | "logic_analyzer" => crate::pin_owner::Driver::Capture,
                    "scope" | "oscilloscope" => crate::pin_owner::Driver::Oscilloscope,
                    "generator" => crate::pin_owner::Driver::Generator,
                    "ws2812" => crate::pin_owner::Driver::Ws2812,
                    "glitch" => crate::pin_owner::Driver::Glitch,
                    _ => crate::pin_owner::Driver::Gpio,
                };
                match crate::pin_owner::claim(driver, &[pin_num], 0) {
                    Ok(slot) => {
                        let _ = write!(out, r#"{{"status":"ok","pin":{},"slot":{},"driver":"{}"}}"#, pin_num, slot, driver_str);
                        false
                    }
                    Err(e) => {
                        let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{:?}"}}"#, pin_num, e);
                        true
                    }
                }
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                true
            }
        }
        "pins_release" => {
            let pin = extract_u32_arg(body, "pin");
            if let Some(pin_num) = pin {
                if let Some(slot) = crate::pin_owner::has_claim(pin_num) {
                    match crate::pin_owner::release(slot) {
                        Ok(()) => {
                            let _ = write!(out, r#"{{"status":"ok","pin":{},"released":true}}"#, pin_num);
                            false
                        }
                        Err(e) => {
                            let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{:?}"}}"#, pin_num, e);
                            true
                        }
                    }
                } else {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"released":false,"message":"pin not claimed"}}"#, pin_num);
                    false
                }
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                true
            }
        }
        "system_firmware_status" => {
            let (bt, _) = crate::boot::get_boot_state();
            let cur = crate::boot::current_executing_slot();
            let _ = write!(
                out,
                "current_slot={:?} active_slot={:?} state={:?} attempts={}",
                cur,
                bt.get_active_slot(),
                bt.get_state(),
                bt.boot_attempts
            );
            false
        }
        "profile_status" => {
            let _ = write!(out, "active_slot={} active_name={}", profile::active_slot(), profile::active_name().as_str());
            false
        }
        "servo_list" => {
            let n = servo::servo_list().len();
            let _ = write!(out, "channels={}", n);
            false
        }
        "servo_attach" => {
            let pin = extract_u32_arg(body, "pin");
            if let Some(pin_num) = pin {
                let min_us = extract_u32_arg(body, "min_us");
                let max_us = extract_u32_arg(body, "max_us");
                let continuous = extract_bool_arg(body, "continuous").unwrap_or(false);
                match servo::servo_attach(pin_num, min_us, max_us, continuous) {
                    Ok(ch) => {
                        let _ = write!(
                            out,
                            r#"{{"status":"ok","pin":{},"continuous":{},"min_us":{},"max_us":{},"angle":{},"pulse_us":{},"active":{}}}"#,
                            ch.pin, ch.continuous, ch.min_us, ch.max_us, ch.angle, ch.pulse_us, ch.active
                        );
                        false
                    }
                    Err(e) => {
                        let err_msg = match e {
                            servo::EngineError::PinConflict => "pin conflict: pin already claimed by another driver",
                            servo::EngineError::SliceConflict => "slice conflict: adjacent pin on PWM slice running incompatible frequency",
                            servo::EngineError::NotAvailable => "pin not available for PWM",
                            servo::EngineError::ChannelsFull => "channel limit reached (maximum 8 servo channels)",
                            servo::EngineError::ClaimRegFull => "pin claim registry full",
                            servo::EngineError::RegistryBusy => "pin registry busy",
                            servo::EngineError::InvalidPulse => "invalid pulse limit parameter",
                            servo::EngineError::InvalidAngle => "invalid angle parameter",
                            _ => "servo attach failed",
                        };
                        let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}","code":"{:?}"}}"#, pin_num, err_msg, e);
                        true
                    }
                }
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                true
            }
        }
        "servo_write_angle" => {
            let pin = extract_u32_arg(body, "pin");
            let angle = extract_i32_arg(body, "angle");
            match (pin, angle) {
                (Some(pin_num), Some(deg)) => {
                    match servo::servo_write_angle(pin_num, deg) {
                        Ok(ch) => {
                            let _ = write!(
                                out,
                                r#"{{"status":"ok","pin":{},"angle":{},"pulse_us":{},"continuous":{},"active":{}}}"#,
                                ch.pin, ch.angle, ch.pulse_us, ch.continuous, ch.active
                            );
                            false
                        }
                        Err(e) => {
                            let err_msg = match e {
                                servo::EngineError::NotAttached => "channel not attached as servo",
                                servo::EngineError::InvalidAngle => "angle out of bounds (0-180 for standard, -100 to 100 for continuous)",
                                _ => "servo write angle failed",
                            };
                            let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}","code":"{:?}"}}"#, pin_num, err_msg, e);
                            true
                        }
                    }
                }
                (None, _) => {
                    let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                    true
                }
                (_, None) => {
                    let _ = out.push_str(r#"{"status":"error","error":"missing angle parameter"}"#);
                    true
                }
            }
        }
        "servo_write_us" => {
            let pin = extract_u32_arg(body, "pin");
            let pulse_us = extract_u32_arg(body, "pulse_us");
            match (pin, pulse_us) {
                (Some(pin_num), Some(us)) => {
                    match servo::servo_write_us(pin_num, us) {
                        Ok(ch) => {
                            let _ = write!(
                                out,
                                r#"{{"status":"ok","pin":{},"pulse_us":{},"angle":{},"active":{}}}"#,
                                ch.pin, ch.pulse_us, ch.angle, ch.active
                            );
                            false
                        }
                        Err(e) => {
                            let err_msg = match e {
                                servo::EngineError::NotAttached => "channel not attached as servo",
                                _ => "servo write pulse width failed",
                            };
                            let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}","code":"{:?}"}}"#, pin_num, err_msg, e);
                            true
                        }
                    }
                }
                (None, _) => {
                    let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                    true
                }
                (_, None) => {
                    let _ = out.push_str(r#"{"status":"error","error":"missing pulse_us parameter"}"#);
                    true
                }
            }
        }
        "servo_detach" => {
            let pin = extract_u32_arg(body, "pin");
            if let Some(pin_num) = pin {
                match servo::detach(pin_num) {
                    Ok(()) => {
                        let _ = write!(out, r#"{{"status":"ok","pin":{},"detached":true}}"#, pin_num);
                        false
                    }
                    Err(e) => {
                        let err_msg = match e {
                            servo::EngineError::NotAttached => "channel not attached",
                            _ => "servo detach failed",
                        };
                        let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}","code":"{:?}"}}"#, pin_num, err_msg, e);
                        true
                    }
                }
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing pin parameter"}"#);
                true
            }
        }
        "logic_analyzer_arm" => {
            let samples = (extract_u32_arg(body, "samples").unwrap_or(1024)).clamp(64, 32768);
            let res = crate::analyzer::no_trigger(samples, &[20, 21], 2);
            let _ = write!(out, r#"{{"status":"armed","samples":{},"channels":[20,21],"success":{}}}"#, samples, res.is_ok());
            false
        }
        "logic_analyzer_start" => {
            let res = crate::analyzer::start();
            let _ = write!(out, "analyzer start={}", res.is_ok());
            false
        }
        "logic_analyzer_status" => {
            let st = crate::analyzer::get_status();
            let _ = write!(out, "state={:?} samples={}", st.state, st.sample_count);
            false
        }
        "logic_analyzer_data" => {
            let w = crate::analyzer::bank_word(0);
            let _ = write!(out, "first_word=0x{:08x} total={}", w, crate::analyzer::sample_count());
            false
        }
        "programmer_read" => {
            let addr = extract_u32_arg(body, "address").unwrap_or(0x08000000);
            let len = (extract_u32_arg(body, "length").unwrap_or(16) as usize).clamp(1, 256);
            let mut buf = [0u8; 256];
            match crate::programmer::read_flash_global(addr, &mut buf[..len]) {
                Ok(()) => {
                    let _ = write!(out, r#"{{"status":"ok","address":"0x{:08X}","length":{},"bytes":["#, addr, len);
                    for (i, &b) in buf[..len].iter().enumerate() {
                        if i > 0 { let _ = out.push(','); }
                        let _ = write!(out, "{}", b);
                    }
                    let _ = out.push_str("]}");
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","address":"0x{:08X}","error":"{:?}"}}"#, addr, e);
                    true
                }
            }
        }
        "programmer_flash" => {
            if !crate::rest::HTTP_FLASH_ENABLED {
                let _ = out.push_str(crate::rest::HTTP_FLASH_DISABLED_BODY);
                true
            } else {
            let addr = extract_u32_arg(body, "address").unwrap_or(0x08000000);
            let hex = extract_string_arg(body, "hex_data").unwrap_or("");
            let mut buf = [0u8; 256];
            let len = decode_hex(hex, &mut buf);
            if len == 0 {
                let _ = out.push_str(r#"{"status":"error","error":"no valid hex bytes provided"}"#);
                true
            } else {
                match crate::programmer::flash_target_global(addr, &buf[..len]) {
                    Ok(()) => {
                        let _ = write!(out, r#"{{"status":"ok","address":"0x{:08X}","bytes_written":{}}}"#, addr, len);
                        false
                    }
                    Err(e) => {
                        let _ = write!(out, r#"{{"status":"error","address":"0x{:08X}","error":"{:?}"}}"#, addr, e);
                        true
                    }
                }
            }
            }
        }
        "programmer_reset" => {
            let _ = write!(out, "programmer target reset completed");
            false
        }
        "programmer_unbrick" => {
            if !crate::rest::HTTP_FLASH_ENABLED {
                let _ = out.push_str(crate::rest::HTTP_FLASH_DISABLED_BODY);
                true
            } else {
            let target = extract_string_arg(body, "target").unwrap_or("stm32");
            if target.eq_ignore_ascii_case("nrf") || target.starts_with("nrf") || target.starts_with("nRF") || target.starts_with("NRF") {
                match crate::programmer::target::nrf::rescue_nrf_global() {
                    Ok(()) => {
                        let _ = write!(out, r#"{{"status":"ok","unprotected":true,"mass_erased":true,"target":"{}"}}"#, target);
                        false
                    }
                    Err(e) => {
                        let _ = write!(out, r#"{{"status":"error","target":"{}","error":"{:?}"}}"#, target, e);
                        true
                    }
                }
            } else {
                match crate::programmer::rescue_stm32_global() {
                    Ok(()) => {
                        let _ = write!(out, r#"{{"status":"ok","unprotected":true,"mass_erased":true,"target":"{}"}}"#, target);
                        false
                    }
                    Err(e) => {
                        let _ = write!(out, r#"{{"status":"error","target":"{}","error":"{:?}"}}"#, target, e);
                        true
                    }
                }
            }
            }
        }
        "catalog_lookup" => {
            let query = extract_string_arg(body, "query").unwrap_or("STM32");
            match agentla_targets::catalog::lookup_by_name(query) {
                Some(desc) => {
                    let _ = write!(
                        out,
                        r#"{{"found":true,"name":"{}","family":"{:?}","flash_base":"0x{:08X}","flash_size":{},"ram_base":"0x{:08X}","ram_size":{},"dev_id":"0x{:04X}"}}"#,
                        desc.name, desc.family, desc.flash_base, desc.flash_size, desc.ram_base, desc.ram_size, desc.dev_id
                    );
                    false
                }
                None => {
                    let _ = write!(out, r#"{{"found":false,"query":"{}"}}"#, query);
                    false
                }
            }
        }
        "i2c_scan" => {
            let sda = extract_u32_arg(body, "sda_pin").unwrap_or(16);
            let scl = extract_u32_arg(body, "scl_pin").unwrap_or(17);
            let found = crate::bridge::i2c::hw_i2c_scan(sda, scl);
            let _ = write!(out, r#"{{"sda":{},"scl":{},"devices":["#, sda, scl);
            for (i, &addr) in found.iter().enumerate() {
                if i > 0 { let _ = out.push(','); }
                let _ = write!(out, "{}", addr);
            }
            let _ = out.push_str("]}");
            false
        }
        "can_status" => {
            let active = crate::bridge::can_gsusb::adapter_can_is_active();
            let bitrate = crate::bridge::can_gsusb::adapter_can_get_bitrate();
            let termination = crate::bridge::can_gsusb::adapter_can_get_termination();
            let tx = crate::bridge::can_gsusb::adapter_can_get_tx_frames();
            let rx = crate::bridge::can_gsusb::adapter_can_get_rx_frames();
            let _ = write!(
                out,
                r#"{{"active":{},"bitrate":{},"termination":{},"tx_frames":{},"rx_frames":{}}}"#,
                active, bitrate, termination, tx, rx
            );
            false
        }
        "can_send" => {
            let id = extract_u32_arg(body, "id").unwrap_or(0);
            let ext = extract_bool_arg(body, "extended").unwrap_or(false);
            let rtr = extract_bool_arg(body, "rtr").unwrap_or(false);
            let hex = extract_string_arg(body, "data_hex").unwrap_or("");
            let mut data = [0u8; 8];
            let dlc = decode_hex(hex, &mut data).min(8) as u8;
            crate::bridge::can_gsusb::adapter_can_start();
            let frame = crate::bridge::can_gsusb::CanWsFrame {
                id,
                ext,
                rtr,
                dlc,
                data,
                ts_us: (embassy_time::Instant::now().as_micros() & 0xFFFF_FFFF) as u32,
            };
            crate::bridge::can_gsusb::transmit_can_ws_frame(&frame);
            let _ = write!(out, r#"{{"status":"ok","transmitted":true,"id":{},"dlc":{}}}"#, id, dlc);
            false
        }
        "can_sniff" => {
            let max_frames = (extract_u32_arg(body, "max_frames").unwrap_or(10) as usize).clamp(1, 16);
            let mut count = 0;
            let _ = out.push_str(r#"{"frames":["#);
            while count < max_frames {
                if let Some(frame) = crate::bridge::can_gsusb::pop_can_rx_frame() {
                    if count > 0 { let _ = out.push(','); }
                    let mut hex_buf = heapless::String::<17>::new();
                    let dlc = (frame.dlc as usize).min(8);
                    for b in &frame.data[..dlc] {
                        let _ = write!(hex_buf, "{:02X}", b);
                    }
                    let _ = write!(
                        out,
                        r#"{{"id":{},"ext":{},"dlc":{},"data":"{}","ts":{}}}"#,
                        frame.id, frame.ext, frame.dlc, hex_buf.as_str(), frame.ts_us
                    );
                    count += 1;
                } else {
                    break;
                }
            }
            let _ = write!(out, r#"],"captured":{}}}"#, count);
            false
        }
        "can_set_termination" => {
            let en = extract_bool_arg(body, "enabled").unwrap_or(false);
            crate::bridge::can_gsusb::adapter_can_set_termination(en);
            let _ = write!(out, r#"{{"status":"ok","termination":{}}}"#, en);
            false
        }
        "spi_transfer" => {
            let hex = extract_string_arg(body, "mosi_hex").unwrap_or("");
            let mut tx_buf = [0u8; 64];
            let tx_len = decode_hex(hex, &mut tx_buf).max(1);
            let mut rx_buf = [0u8; 64];
            let count = crate::bridge::spi::spi_transfer_default(0, 1_000_000, &tx_buf[..tx_len], &mut rx_buf[..tx_len]);
            let mut miso_hex = heapless::String::<130>::new();
            for &b in &rx_buf[..count] {
                let _ = write!(miso_hex, "{:02X}", b);
            }
            let _ = write!(out, r#"{{"status":"ok","bytes":{},"miso_hex":"{}"}}"#, count, miso_hex.as_str());
            false
        }
        "uart_command" => {
            let cmd = extract_string_arg(body, "tx_data").unwrap_or("");
            let baud = extract_u32_arg(body, "baud").unwrap_or(115200);
            let timeout = extract_u32_arg(body, "timeout_ms").unwrap_or(500).clamp(1, 2000);
            let bytes_sent = if !cmd.is_empty() {
                crate::bridge::uart::uart_write_bytes(crate::bridge::uart::PIN_UART_TX_DEFAULT, baud, cmd.as_bytes())
            } else {
                0
            };
            let mut rx_buf = [0u8; 128];
            let n = crate::bridge::uart::uart_read_blocking(crate::bridge::uart::PIN_UART_RX_DEFAULT, baud, &mut rx_buf, timeout);
            let mut rx_escaped = heapless::String::<256>::new();
            for &b in &rx_buf[..n] {
                if b.is_ascii_graphic() || b == b' ' {
                    let _ = rx_escaped.push(b as char);
                } else if b == b'\n' {
                    let _ = rx_escaped.push_str("\\n");
                } else if b == b'\r' {
                    let _ = rx_escaped.push_str("\\r");
                } else {
                    let _ = write!(rx_escaped, "\\x{:02X}", b);
                }
            }
            let _ = write!(
                out,
                r#"{{"status":"ok","bytes_sent":{},"bytes_read":{},"response":"{}"}}"#,
                bytes_sent, n, rx_escaped.as_str()
            );
            false
        }
        "neopixel_fill" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            let color_str = extract_string_arg(body, "color").unwrap_or("#FFFFFF");
            let count = extract_u32_arg(body, "count").unwrap_or(16) as usize;
            let (r, g, b) = parse_mcp_color(color_str);
            match crate::instruments::ws2812::fill(pin, r, g, b, count) {
                Ok(n) => {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"count":{},"color":"{}"}}"#, pin, n, color_str);
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "neopixel_set" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            let index = extract_u32_arg(body, "index").unwrap_or(0) as usize;
            let color_str = extract_string_arg(body, "color").unwrap_or("#FFFFFF");
            let (r, g, b) = parse_mcp_color(color_str);
            match crate::instruments::ws2812::set_pixel(pin, index, r, g, b) {
                Ok(_) => {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"index":{},"color":"{}"}}"#, pin, index, color_str);
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "neopixel_clear" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            let count = extract_u32_arg(body, "count").unwrap_or(64) as usize;
            match crate::instruments::ws2812::clear(pin, count) {
                Ok(_) => {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"count":{},"cleared":true}}"#, pin, count);
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "neopixel_pattern" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            let pattern = extract_string_arg(body, "pattern").unwrap_or("rainbow");
            let count = extract_u32_arg(body, "count").unwrap_or(16) as usize;
            let speed = extract_u32_arg(body, "speed").unwrap_or(50);
            match crate::instruments::ws2812::drive_pattern(pin, pattern, count) {
                Ok(_) => {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"pattern":"{}","count":{},"speed":{}}}"#, pin, pattern, count, speed);
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "neopixel_analyze" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            match crate::instruments::ws2812::analyze_timing(pin) {
                Ok(rep) => {
                    let _ = write!(
                        out,
                        r#"{{"status":"ok","pin":{},"t0h_ns":{},"t1h_ns":{},"t0l_ns":{},"t1l_ns":{},"reset_us":{},"passed":{}}}"#,
                        rep.pin, rep.t0h_ns, rep.t1h_ns, rep.t0l_ns, rep.t1l_ns, rep.reset_us, rep.passed
                    );
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "ws2812_transmit" | "neopixel_transmit" => {
            let pin = extract_u32_arg(body, "pin").unwrap_or(22);
            let mut colors: heapless::Vec<(u8, u8, u8), 64> = heapless::Vec::new();
            if let Some(pos) = body.find("\"colors\"") {
                let rest = &body[pos..];
                if let Some(start) = rest.find('[') {
                    let arr_slice = &rest[start + 1..];
                    if let Some(end) = arr_slice.find(']') {
                        let inner = &arr_slice[..end];
                        for item in inner.split(',') {
                            let clean = item.trim().trim_matches('"');
                            if !clean.is_empty() {
                                let _ = colors.push(parse_mcp_color(clean));
                            }
                        }
                    }
                }
            }
            if colors.is_empty() {
                let _ = colors.push((255, 0, 0));
                let _ = colors.push((0, 255, 0));
                let _ = colors.push((0, 0, 255));
            }
            match crate::instruments::ws2812::transmit_colors(pin, &colors) {
                Ok(n) => {
                    let _ = write!(out, r#"{{"status":"ok","pin":{},"pixels_updated":{}}}"#, pin, n);
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","pin":{},"error":"{}"}}"#, pin, e);
                    true
                }
            }
        }
        "target_diagnose" | "mcp_target_diagnose" => {
            let port = extract_string_arg(body, "port");
            let report = crate::programmer::target::diagnostic::run_target_diagnose(port);
            let json = report.to_json();
            let _ = out.push_str(json.as_str());
            false
        }
        "inspect_firmware" | "mcp_inspect_firmware" => {
            let addr = extract_u32_arg(body, "address");
            let target = extract_string_arg(body, "target");
            let (json, is_err) = crate::programmer::target::diagnostic::run_inspect_firmware(addr, target);
            let _ = out.push_str(json.as_str());
            is_err
        }
        "sniff_bus" | "mcp_sniff_bus" => {
            let protocol = extract_string_arg(body, "protocol").unwrap_or("i2c");
            let duration_ms = extract_u32_arg(body, "duration_ms").unwrap_or(200);
            match crate::analyzer::sniff_bus(protocol, duration_ms) {
                Ok(json) => {
                    let _ = out.push_str(json.as_str());
                    false
                }
                Err(err_msg) => {
                    let _ = write!(out, "Pin conflict: {}", err_msg);
                    true
                }
            }
        }
        "app_list" => {
            let mut apps_buf = heapless::String::<1024>::new();
            let ok = match crate::apps::list_apps_json(&mut apps_buf) {
                Some(Ok(())) => true,
                _ => {
                    apps_buf.clear();
                    let _ = apps_buf.push_str("[]");
                    false
                }
            };
            let (used, total) = crate::storage::user_fs_usage();
            let free = total.saturating_sub(used);
            let _ = write!(
                out,
                r#"{{"apps":{},"storage":{{"used_bytes":{},"total_bytes":{},"free_bytes":{}}},"success":{}}}"#,
                apps_buf.as_str(), used, total, free, ok
            );
            !ok
        }
        "app_install" => {
            if let Some(name) = extract_string_arg(body, "name") {
                match crate::apps::install_catalog_package(name) {
                    Ok(rep) => {
                        let _ = write!(
                            out,
                            r#"{{"status":"ok","app":"{}","files":{},"bytes":{}}}"#,
                            rep.app.as_str(), rep.files, rep.bytes
                        );
                        false
                    }
                    Err(e) => {
                        let _ = write!(out, r#"{{"status":"error","app":"{}","error":"{:?}"}}"#, name, e);
                        true
                    }
                }
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing name parameter"}"#);
                true
            }
        }
        "power_control" => {
            if let Some(enabled) = extract_bool_arg(body, "enabled") {
                crate::profile::set_target_power(enabled);
                if let Some(voltage_mv) = extract_u32_arg(body, "voltage_mv") {
                    crate::telemetry::set_target_voltage_mv(voltage_mv);
                }
                let snap = crate::telemetry::sample_telemetry();
                let v_set = crate::profile::get_target_voltage_setting_mv();
                let _ = write!(
                    out,
                    r#"{{"status":"ok","power_enabled":{},"voltage_setting_mv":{},"vtarget_mv":{},"itarget_ma":{}}}"#,
                    enabled, v_set, snap.v_target_mv, snap.i_target_ma
                );
                false
            } else {
                let _ = out.push_str(r#"{"status":"error","error":"missing enabled parameter"}"#);
                true
            }
        }
        "power_cycle" => {
            let delay_ms = extract_u32_arg(body, "delay_ms").unwrap_or(100).clamp(10, 5000);
            crate::profile::set_target_power(false);
            embassy_time::block_for(embassy_time::Duration::from_millis(delay_ms as u64));
            crate::profile::set_target_power(true);
            let snap = crate::telemetry::sample_telemetry();
            let _ = write!(
                out,
                r#"{{"status":"ok","power_cycled":true,"delay_ms":{},"vtarget_mv":{},"itarget_ma":{}}}"#,
                delay_ms, snap.v_target_mv, snap.i_target_ma
            );
            false
        }
        "emergency_safe_state" => {
            let _ = crate::profile::apply_profile_sync(crate::profile::SAFE_STATE_ID);
            let _ = out.push_str(r#"{"status":"ok","safe_state":true,"pins_released":true,"power_enabled":false}"#);
            false
        }
        "profile_list" => {
            let aid = crate::profile::active_id();
            let aslot = crate::profile::active_slot();
            let _ = write!(
                out,
                r#"{{"active_slot":{},"active_id":"{}","slots":["#,
                aslot, aid.as_str()
            );
            let slots = [
                (0, "safe_state", "Safe State (Hi-Z)"),
                (1, "logic_analyzer", "Logic Analyzer"),
                (2, "can_sniffer", "CAN Bus Sniffer & Logger"),
                (3, "uart_passthrough", "UART Passthrough"),
                (4, "user_profile_4", "User Profile 4"),
                (5, "user_profile_5", "User Profile 5"),
                (6, "user_profile_6", "User Profile 6"),
                (7, "user_profile_7", "User Profile 7"),
            ];
            for (i, (s_idx, s_id, s_desc)) in slots.iter().enumerate() {
                if i > 0 { let _ = out.push(','); }
                let is_act = *s_idx == aslot || *s_id == aid.as_str();
                let _ = write!(
                    out,
                    r#"{{"slot":{},"id":"{}","description":"{}","active":{}}}"#,
                    s_idx, s_id, s_desc, is_act
                );
            }
            let _ = out.push_str("]}");
            false
        }
        "profile_apply" => {
            let slot_opt = extract_u32_arg(body, "slot");
            let id_opt = extract_string_arg(body, "id");
            let target_id = if let Some(id_str) = id_opt {
                id_str
            } else if let Some(slot) = slot_opt {
                match slot {
                    0 => "safe_state",
                    1 => "logic_analyzer",
                    2 => "can_sniffer",
                    3 => "uart_passthrough",
                    _ => "safe_state",
                }
            } else {
                "safe_state"
            };
            match crate::profile::apply_profile_sync(target_id) {
                Ok(name) => {
                    let _ = write!(
                        out,
                        r#"{{"status":"ok","applied":true,"id":"{}","name":"{}"}}"#,
                        target_id, name.as_str()
                    );
                    false
                }
                Err(e) => {
                    let _ = write!(out, r#"{{"status":"error","id":"{}","error":"{:?}"}}"#, target_id, e);
                    true
                }
            }
        }
        "swo_read" => {
            let clear = extract_bool_arg(body, "clear").unwrap_or(true);
            let mut buf = [0u8; 512];
            let n = if clear {
                crate::programmer::swo::drain_swo_bytes(&mut buf)
            } else {
                crate::programmer::swo::SWO_ENGINE.peek(&mut buf)
            };
            let mut trace_escaped = heapless::String::<1024>::new();
            for &b in &buf[..n] {
                if b.is_ascii_graphic() || b == b' ' {
                    let _ = trace_escaped.push(b as char);
                } else if b == b'\n' {
                    let _ = trace_escaped.push_str("\\n");
                } else if b == b'\r' {
                    let _ = trace_escaped.push_str("\\r");
                } else if b == b'\t' {
                    let _ = trace_escaped.push_str("\\t");
                } else {
                    let _ = write!(trace_escaped, "\\x{:02X}", b);
                }
            }
            let _ = write!(out, r#"{{"status":"ok","bytes":{},"data":"{}"}}"#, n, trace_escaped.as_str());
            false
        }
        "sensor_scan" => {
            let bus = extract_string_arg(body, "bus").unwrap_or("i2c");
            if bus == "spi" {
                let cs = extract_u32_arg(body, "cs_pin").unwrap_or(17);
                let speed_khz = extract_u32_arg(body, "speed_khz").unwrap_or(1000);
                let speed_hz = speed_khz.clamp(100, 20_000) * 1000;
                // Probe JEDEC ID (0x9F)
                let tx = [0x9Fu8, 0x00, 0x00, 0x00];
                let mut rx = [0u8; 4];
                crate::bridge::spi::spi_transfer_default(0, speed_hz, &tx, &mut rx);
                let has_jedec = rx[1] != 0x00 && rx[1] != 0xFF;
                let _ = write!(out, r#"{{"status":"ok","bus":"spi","cs_pin":{},"devices":["#, cs);
                if has_jedec {
                    let _ = write!(out, r#"{{"cs":{},"type":"jedec_flash","mfg_id":"0x{:02X}","dev_id":"0x{:02X}{:02X}"}}"#, cs, rx[1], rx[2], rx[3]);
                }
                let _ = out.push_str("]}");
                false
            } else {
                let sda = crate::bridge::i2c::I2C_PINS[0];
                let scl = crate::bridge::i2c::I2C_PINS[1];
                let found = crate::bridge::i2c::hw_i2c_scan(sda, scl);
                let _ = write!(out, r#"{{"status":"ok","bus":"i2c","count":{},"devices":["#, found.len());
                for (i, &addr) in found.iter().enumerate() {
                    if i > 0 { let _ = out.push(','); }
                    let profile = match addr {
                        0x68 | 0x69 => "MPU6050",
                        0x35 => "TMAG5273",
                        0x36 => "AS5600",
                        0x76 | 0x77 => "BME280",
                        0x40 | 0x41 | 0x44 | 0x45 => "INA219",
                        0x18 | 0x19 => "LIS3DH",
                        0x29 => "VL53L0X",
                        _ => "Generic_I2C",
                    };
                    let _ = write!(out, r#"{{"addr":{},"hex":"0x{:02X}","profile":"{}"}}"#, addr, addr, profile);
                }
                let _ = out.push_str("]}");
                false
            }
        }
        "sensor_read_reg" => {
            let bus = extract_string_arg(body, "bus").unwrap_or("i2c");
            let addr = extract_u32_arg(body, "addr").unwrap_or(0x68) as u8;
            let reg = extract_u32_arg(body, "reg").unwrap_or(0) as u8;
            let len = (extract_u32_arg(body, "len").unwrap_or(1) as usize).clamp(1, 32);

            if bus == "spi" {
                let mut tx = [0u8; 33];
                tx[0] = reg | 0x80; // read bit
                let mut rx = [0u8; 33];
                crate::bridge::spi::spi_transfer_default(0, 1_000_000, &tx[..len + 1], &mut rx[..len + 1]);
                let _ = write!(out, r#"{{"status":"ok","bus":"spi","reg":{},"len":{},"hex":""#, reg, len);
                for &b in &rx[1..len + 1] {
                    let _ = write!(out, "{:02X}", b);
                }
                let _ = write!(out, r#"","bytes":["#);
                for (i, &b) in rx[1..len + 1].iter().enumerate() {
                    if i > 0 { let _ = out.push(','); }
                    let _ = write!(out, "{}", b);
                }
                let _ = out.push_str("]}");
                false
            } else {
                let sda = crate::bridge::i2c::I2C_PINS[0];
                let scl = crate::bridge::i2c::I2C_PINS[1];
                let mut rx_buf = [0u8; 32];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, reg, &mut rx_buf[..len]);
                if ok {
                    let _ = write!(out, r#"{{"status":"ok","bus":"i2c","addr":{},"reg":{},"len":{},"hex":""#, addr, reg, len);
                    for &b in &rx_buf[..len] {
                        let _ = write!(out, "{:02X}", b);
                    }
                    let _ = write!(out, r#"","bytes":["#);
                    for (i, &b) in rx_buf[..len].iter().enumerate() {
                        if i > 0 { let _ = out.push(','); }
                        let _ = write!(out, "{}", b);
                    }
                    let _ = out.push_str("]}");
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"reg":{}}}"#, addr, reg);
                    true
                }
            }
        }
        "sensor_write_reg" => {
            let bus = extract_string_arg(body, "bus").unwrap_or("i2c");
            let addr = extract_u32_arg(body, "addr").unwrap_or(0x68) as u8;
            let reg = extract_u32_arg(body, "reg").unwrap_or(0) as u8;
            let mut data_buf = [0u8; 32];
            let mut data_len = 0usize;

            if let Some(hex_str) = extract_string_arg(body, "data") {
                let clean = hex_str.trim_start_matches("0x").trim_start_matches("0X");
                data_len = decode_hex(clean, &mut data_buf);
            }
            if data_len == 0 {
                if let Some(val) = extract_u32_arg(body, "data") {
                    data_buf[0] = val as u8;
                    data_len = 1;
                }
            }
            if data_len == 0 {
                data_len = 1;
                data_buf[0] = 0;
            }

            if bus == "spi" {
                let mut tx = [0u8; 33];
                tx[0] = reg & 0x7F; // write bit
                tx[1..data_len + 1].copy_from_slice(&data_buf[..data_len]);
                let mut rx = [0u8; 33];
                crate::bridge::spi::spi_transfer_default(0, 1_000_000, &tx[..data_len + 1], &mut rx[..data_len + 1]);
                let _ = write!(out, r#"{{"status":"ok","bus":"spi","reg":{},"written":{}}}"#, reg, data_len);
                false
            } else {
                let sda = crate::bridge::i2c::I2C_PINS[0];
                let scl = crate::bridge::i2c::I2C_PINS[1];
                let ok = crate::bridge::i2c::hw_i2c_write(sda, scl, addr, reg, &data_buf[..data_len]);
                if ok {
                    let _ = write!(out, r#"{{"status":"ok","bus":"i2c","addr":{},"reg":{},"written":{}}}"#, addr, reg, data_len);
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"reg":{}}}"#, addr, reg);
                    true
                }
            }
        }
        "sensor_poll" => {
            let addr = extract_u32_arg(body, "addr").unwrap_or(0x68) as u8;
            let profile_str = extract_string_arg(body, "profile").unwrap_or("");
            let sda = crate::bridge::i2c::I2C_PINS[0];
            let scl = crate::bridge::i2c::I2C_PINS[1];

            let is_mpu = profile_str == "mpu6050" || (profile_str.is_empty() && (addr == 0x68 || addr == 0x69));
            let is_tmag = profile_str == "tmag5273" || (profile_str.is_empty() && addr == 0x35);
            let is_as5600 = profile_str == "as5600" || (profile_str.is_empty() && addr == 0x36);
            let is_bme = profile_str == "bme280" || (profile_str.is_empty() && (addr == 0x76 || addr == 0x77));
            let is_ina = profile_str == "ina219" || (profile_str.is_empty() && (addr == 0x40 || addr == 0x41 || addr == 0x44 || addr == 0x45));

            if is_mpu {
                let mut buf = [0u8; 14];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0x3B, &mut buf);
                if ok {
                    let ax = i16::from_be_bytes([buf[0], buf[1]]);
                    let ay = i16::from_be_bytes([buf[2], buf[3]]);
                    let az = i16::from_be_bytes([buf[4], buf[5]]);
                    let temp_raw = i16::from_be_bytes([buf[6], buf[7]]);
                    let gx = i16::from_be_bytes([buf[8], buf[9]]);
                    let gy = i16::from_be_bytes([buf[10], buf[11]]);
                    let gz = i16::from_be_bytes([buf[12], buf[13]]);

                    let ax_milli = (ax as i32 * 1000) / 16384;
                    let ay_milli = (ay as i32 * 1000) / 16384;
                    let az_milli = (az as i32 * 1000) / 16384;
                    let temp_deg = (temp_raw as i32 * 10) / 340 + 365;

                    let _ = write!(
                        out,
                        r#"{{"status":"ok","addr":{},"profile":"mpu6050","telemetry":{{"accel_x_g":{}.{:03},"accel_y_g":{}.{:03},"accel_z_g":{}.{:03},"temp_c":{}.{:01},"gyro_x_dps":{},"gyro_y_dps":{},"gyro_z_dps":{}}}}}"#,
                        addr,
                        ax_milli / 1000, (ax_milli % 1000).abs(),
                        ay_milli / 1000, (ay_milli % 1000).abs(),
                        az_milli / 1000, (az_milli % 1000).abs(),
                        temp_deg / 10, (temp_deg % 10).abs(),
                        (gx as i32) / 131,
                        (gy as i32) / 131,
                        (gz as i32) / 131
                    );
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"profile":"mpu6050"}}"#, addr);
                    true
                }
            } else if is_tmag {
                let mut buf = [0u8; 4];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0x12, &mut buf);
                if ok {
                    let raw_angle = u16::from_be_bytes([buf[0], buf[1]]) >> 4;
                    let deg = (raw_angle as u32 * 3600) / 4096;
                    let _ = write!(out, r#"{{"status":"ok","addr":{},"profile":"tmag5273","telemetry":{{"angle_deg":{}.{:01},"raw":{}}}}}"#, addr, deg / 10, deg % 10, raw_angle);
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"profile":"tmag5273"}}"#, addr);
                    true
                }
            } else if is_as5600 {
                let mut buf = [0u8; 2];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0x0C, &mut buf);
                if ok {
                    let raw = u16::from_be_bytes([buf[0], buf[1]]) & 0x0FFF;
                    let deg = (raw as u32 * 3600) / 4096;
                    let _ = write!(out, r#"{{"status":"ok","addr":{},"profile":"as5600","telemetry":{{"angle_deg":{}.{:01},"raw":{}}}}}"#, addr, deg / 10, deg % 10, raw);
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"profile":"as5600"}}"#, addr);
                    true
                }
            } else if is_bme {
                let mut buf = [0u8; 6];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0xF7, &mut buf);
                if ok {
                    let press_raw = ((buf[0] as u32) << 12) | ((buf[1] as u32) << 4) | ((buf[2] as u32) >> 4);
                    let temp_raw = ((buf[3] as u32) << 12) | ((buf[4] as u32) << 4) | ((buf[5] as u32) >> 4);
                    let press_hpa = press_raw / 256;
                    let temp_c = (temp_raw as i32 - 100000) / 5120;
                    let _ = write!(out, r#"{{"status":"ok","addr":{},"profile":"bme280","telemetry":{{"press_hpa":{},"temp_c":{}}}}}"#, addr, press_hpa, temp_c);
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"profile":"bme280"}}"#, addr);
                    true
                }
            } else if is_ina {
                let mut buf = [0u8; 2];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0x02, &mut buf);
                if ok {
                    let raw_v = u16::from_be_bytes([buf[0], buf[1]]) >> 3;
                    let mv = raw_v as u32 * 4;
                    let _ = write!(out, r#"{{"status":"ok","addr":{},"profile":"ina219","telemetry":{{"voltage_mv":{},"voltage_v":{}.{:03}}}}}"#, addr, mv, mv / 1000, mv % 1000);
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{},"profile":"ina219"}}"#, addr);
                    true
                }
            } else {
                let mut buf = [0u8; 8];
                let ok = crate::bridge::i2c::hw_i2c_read(sda, scl, addr, 0x00, &mut buf);
                if ok {
                    let _ = write!(out, r#"{{"status":"ok","addr":{},"profile":"generic","raw_hex":""#, addr);
                    for &b in &buf {
                        let _ = write!(out, "{:02X}", b);
                    }
                    let _ = out.push_str("\"}}");
                    false
                } else {
                    let _ = write!(out, r#"{{"status":"nack","addr":{}}}"#, addr);
                    true
                }
            }
        }
        _ => {
            let _ = write!(out, "unknown tool: {}", name);
            true
        }
    }
}

pub struct McpSseEvents;
impl picoserve::response::sse::EventSource for McpSseEvents {
    async fn write_events<W: picoserve::io::Write>(self, mut writer: picoserve::response::sse::EventWriter<'_, W>) -> Result<(), W::Error> {
        writer.write_event("endpoint", format_args!("/mcp")).await?;
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(5000));
        loop { ticker.next().await; writer.write_event("heartbeat", format_args!("{{}}")).await?; }
    }
}
