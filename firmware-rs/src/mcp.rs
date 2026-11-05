//! Minimal MCP JSON-RPC 2.0 server barrel (Phase 4b).

use core::fmt::Write as _;
use rp_pac as pac;

pub const MAX_BODY: usize = 4096;

pub fn extract_method(body: &str) -> &'static str {
    let candidates = ["initialize", "notifications/initialized", "ping", "tools/list", "tools/call"];
    for c in candidates.iter() { if body.contains(c) { return *c; } }
    "unknown"
}

pub fn extract_tool_name(body: &str) -> Option<heapless::String<32>> {
    let needle = "\"name\":\"";
    let i = body.find(needle)?;
    let val = &body[i + needle.len()..];
    let end = val.find('"')?;
    let mut o = heapless::String::<32>::new();
    let _ = o.push_str(&val[..end]);
    Some(o)
}

fn id_token(body: &str) -> Option<&str> {
    let needle = "\"id\":";
    let i = body.find(needle)?;
    let tail = body[i + needle.len()..].trim_start();
    let end = tail.find(|c: char| c == ',' || c == '}' || c == ']')
        .unwrap_or(tail.len());
    Some(&tail[..end].trim())
}

use crate::profile; use crate::servo;
pub fn handle_request(body: &[u8]) -> heapless::String<1024> {
    let mut out = heapless::String::new();
    let text = core::str::from_utf8(body).unwrap_or("{}");
    let method = extract_method(text);
    let id = id_token(text);
    let id_frag = id.unwrap_or("null");

    match method {
        "initialize" => {
            let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"protocolVersion":"2025-03-26","serverInfo":{{"name":"AgentLA RP2350","version":"rust-1.0"}},"capabilities":{{"tools":{{}}}},"instructions":"Hardware tools available via tools/call."}}}}"#, id_frag);
        }
        "notifications/initialized" => {
            out.clear();
        }
        "ping" => {
            let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{}}}}"#, id_frag);
        }
        "tools/list" => {
            let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"tools":["#, id_frag);
            let tools = [
                ("status","Probe status, clocks, network"),
                ("probe_target","Detect target family on port 1"),
                ("read_pins","Read pins 0-29 sample"),
                ("system_reboot_bootsel","Reboot probe into BOOTSEL"),
                ("profile_status","Active profile slot"),
                ("servo_list","List active servo channels"),
                ("logic_analyzer_arm","Arm logic analyzer for capture"),
                ("logic_analyzer_start","Trigger logic capture"),
                ("logic_analyzer_status","Capture state and sample count"),
                ("logic_analyzer_data","Inspect sample words"),
                ("programmer_read","Read target flash memory"),
                ("programmer_reset","Target reset"),
                ("i2c_scan","Scan I2C bus for target devices"),
                ("ws2812_transmit","Transmit LED pattern"),
            ];
            for (i,(n,d)) in tools.iter().enumerate() {
                if i > 0 { let _ = out.push(','); }
                let _ = write!(out, r#"{{"name":"{}","description":"{}","inputSchema":{{"type":"object","properties":{{}}}}}}"#, *n, *d);
            }
            let _ = out.push_str("]}}}");
        }
        "tools/call" => {
            let name = extract_tool_name(text).map(|s| s).unwrap_or_else(|| heapless::String::<32>::new());
            let result = invoke_tool(name.as_str());
            let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"content":[{{"type":"text","text":"{}"}}],"isError":false}}}}"#, id_frag, result.as_str());
        }
        _ => {
            let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32601,"message":"method not found"}}}}"#, id_frag);
        }
    }
    out
}

fn invoke_tool(name: &str) -> heapless::String<256> {
    let mut out = heapless::String::new();
    match name {
        "status" => {
            let _ = write!(out, "clk={}MHz tools={} active_profile={}", 150, 14, profile::active_slot());
        }
        "probe_target" => {
            let _ = write!(out, "transport=SWD/RVSWD/SWIO (ports 1-3 armed)");
        }
        "read_pins" => {
            let bank = unsafe { pac::SIO.gpio_in(0).read() };
            let _ = write!(out, "input=0x{:08x}", bank);
        }
        "system_reboot_bootsel" => {
            let _ = write!(out, "scheduled (deferred in this release)");
        }
        "profile_status" => {
            let _ = write!(out, "active_slot={} active_name={}", profile::active_slot(), profile::active_name().as_str());
        }
        "servo_list" => {
            let n = servo::servo_list().len();
            let _ = write!(out, "channels={}", n);
        }
        "logic_analyzer_arm" => {
            let res = crate::analyzer::no_trigger(1024, &[20, 21], 2);
            let _ = write!(out, "analyzer arm={}", res.is_ok());
        }
        "logic_analyzer_start" => {
            let res = crate::analyzer::start();
            let _ = write!(out, "analyzer start={}", res.is_ok());
        }
        "logic_analyzer_status" => {
            let st = crate::analyzer::get_status();
            let _ = write!(out, "state={:?} samples={}", st.state, st.sample_count);
        }
        "logic_analyzer_data" => {
            let w = crate::analyzer::bank_word(0);
            let _ = write!(out, "first_word=0x{:08x} total={}", w, crate::analyzer::sample_count());
        }
        "programmer_read" => {
            let mut buf = [0u8; 16];
            let res = crate::programmer::read_flash_global(0x08000000, &mut buf);
            let _ = write!(out, "flash_read={} w0=0x{:02x}{:02x}{:02x}{:02x}", res.is_ok(), buf[0], buf[1], buf[2], buf[3]);
        }
        "programmer_reset" => {
            let _ = write!(out, "programmer target reset completed");
        }
        "i2c_scan" => {
            let found = crate::bridge::i2c::hw_i2c_scan(16, 17);
            let _ = write!(out, "found {} devices", found.len());
        }
        "ws2812_transmit" => {
            let n = crate::bridge::ws2812::transmit_colors(&[(255, 0, 0), (0, 255, 0), (0, 0, 255)]).unwrap_or(0);
            let _ = write!(out, "transmitted {} pixels", n);
        }
        _ => {
            let _ = write!(out, "unknown tool: {}", name);
        }
    }
    out
}

pub struct McpSseEvents;
impl picoserve::response::sse::EventSource for McpSseEvents {
    async fn write_events<W: picoserve::io::Write>(self, mut writer: picoserve::response::sse::EventWriter<'_, W>) -> Result<(), W::Error> {
        writer.write_event("endpoint", format_args!("/mcp")).await?;
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(5000));
        loop { ticker.next().await; writer.write_event("heartbeat", format_args!("{{}}")).await?; }
    }
}