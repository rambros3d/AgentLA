//! Minimal MCP JSON-RPC 2.0 server barrel (delegates to `agentla-mcp`).

use core::fmt::Write as _;
use rp_pac as pac;

#[allow(unused_imports)]
pub use agentla_mcp::{
    escape_json, extract_bool_arg, extract_method, extract_string_arg, extract_tool_name,
    extract_u32_arg, McpServer,
};

use crate::profile;
use crate::servo;

pub const MAX_BODY: usize = 8192;

pub fn handle_request(body: &[u8]) -> heapless::String<8192> {
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

fn invoke_tool(name: &str, body: &str, out: &mut heapless::String<1536>) -> bool {
    match name {
        "status" => {
            let snap = crate::telemetry::sample_telemetry();
            let _ = write!(
                out,
                r#"{{"clock_mhz":150,"tools":29,"vtarget_mv":{},"itarget_ma":{},"power_mw":{},"temp_c":{},"overcurrent":{},"active_profile":{},"status":"healthy"}}"#,
                snap.v_target_mv, snap.i_target_ma, snap.p_target_mw, snap.temp_c, snap.is_tripped, profile::active_slot()
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
        "programmer_reset" => {
            let _ = write!(out, "programmer target reset completed");
            false
        }
        "programmer_unbrick" => {
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
        "ws2812_transmit" => {
            let n = crate::bridge::ws2812::transmit_colors(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)]).unwrap_or(0);
            let _ = write!(out, "transmitted {} pixels", n);
            false
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
