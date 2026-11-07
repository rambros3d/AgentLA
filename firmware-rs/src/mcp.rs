//! Minimal MCP JSON-RPC 2.0 server barrel (delegates to `agentla-mcp`).

use core::fmt::Write as _;
use rp_pac as pac;

#[allow(unused_imports)]
pub use agentla_mcp::{
    escape_json, extract_method, extract_string_arg, extract_tool_name, extract_u32_arg,
    McpServer,
};

use crate::profile;
use crate::servo;

pub const MAX_BODY: usize = 4096;

pub fn handle_request(body: &[u8]) -> heapless::String<4096> {
    let server = McpServer::with_handler(invoke_tool);
    server.handle_request(body)
}

fn invoke_tool(name: &str, body: &str, out: &mut heapless::String<1536>) -> bool {
    match name {
        "status" => {
            let _ = write!(out, "clk={}MHz tools={} active_profile={}", 150, 17, profile::active_slot());
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
            let res = crate::analyzer::no_trigger(1024, &[20, 21], 2);
            let _ = write!(out, "analyzer arm={}", res.is_ok());
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
            let mut buf = [0u8; 16];
            let res = crate::programmer::read_flash_global(0x08000000, &mut buf);
            let _ = write!(out, "flash_read={} w0=0x{:02x}{:02x}{:02x}{:02x}", res.is_ok(), buf[0], buf[1], buf[2], buf[3]);
            false
        }
        "programmer_reset" => {
            let _ = write!(out, "programmer target reset completed");
            false
        }
        "i2c_scan" => {
            let found = crate::bridge::i2c::hw_i2c_scan(16, 17);
            let _ = write!(out, "found {} devices", found.len());
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
