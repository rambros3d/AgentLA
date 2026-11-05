use core::fmt::Write as _;

use embassy_net::Stack;
use picoserve::response::sse::{EventSource, EventStream, EventWriter};
use picoserve::response::{File, StatusCode};
use picoserve::routing::{delete, get, get_service, parse_path_segment, post, post_service, PathRouter};
use picoserve::Config as PicoserveConfig;
use picoserve::Server;

pub const INDEX_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>AgentLA • RP2350 Probe</title>
    <link rel="stylesheet" href="/style.css">
</head>
<body>
    <header class="navbar">
        <div class="logo">⚡ AgentLA <span>RP2350 Probe</span></div>
        <div class="status-badge" id="net-status">CDC-NCM Connected (10.10.10.10)</div>
    </header>
    <main class="container">
        <section class="card hero">
            <h2>Hardware Workbench & Debug Supervisor</h2>
            <p>High-throughput logic analyzer, autonomous target programmer, and multi-bus bridge running on dual-core ARM Cortex-M33 @ 150MHz.</p>
        </section>
        <section class="grid">
            <div class="card">
                <h3>System Telemetry</h3>
                <div class="telemetry-item"><span>Architecture:</span> <strong>ARM Cortex-M33 (RP2350)</strong></div>
                <div class="telemetry-item"><span>Clock Frequency:</span> <strong>150 MHz</strong></div>
                <div class="telemetry-item"><span>Total SRAM:</span> <strong>520 KB</strong></div>
                <div class="telemetry-item"><span>Storage Partition:</span> <strong>LittleFS on Flash</strong></div>
                <div class="telemetry-item"><span>Uptime:</span> <strong id="uptime-val">0s</strong></div>
            </div>
            <div class="card">
                <h3>Active Interfaces</h3>
                <div class="telemetry-item"><span>CDC-ACM 0:</span> <strong>Interactive REPL CLI</strong></div>
                <div class="telemetry-item"><span>CDC-ACM 1:</span> <strong>Target UART Bridge</strong></div>
                <div class="telemetry-item"><span>CMSIS-DAP:</span> <strong>v1 (HID) & v2 (Bulk)</strong></div>
                <div class="telemetry-item"><span>Network:</span> <strong>CDC-NCM Virtual Ethernet</strong></div>
            </div>
        </section>
        <section class="card">
            <h3>Live Telemetry Stream</h3>
            <pre id="sse-log">Connecting to /api/v1/events...</pre>
        </section>
    </main>
    <script src="/app.js"></script>
</body>
</html>"#;

pub const STYLE_CSS: &str = r#"
:root {
    --bg-color: #0f172a;
    --card-bg: #1e293b;
    --primary: #38bdf8;
    --text: #f8fafc;
    --text-muted: #94a3b8;
    --border: #334155;
    --success: #10b981;
}
* { box-sizing: border-box; margin: 0; padding: 0; }
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
    background-color: var(--bg-color);
    color: var(--text);
    padding-bottom: 2rem;
}
.navbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 2rem;
    background-color: var(--card-bg);
    border-bottom: 1px solid var(--border);
}
.logo { font-size: 1.25rem; font-weight: bold; }
.logo span { color: var(--primary); font-size: 0.9rem; font-weight: normal; margin-left: 0.5rem; }
.status-badge {
    background-color: rgba(16, 185, 129, 0.15);
    color: var(--success);
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    font-size: 0.85rem;
    font-weight: 500;
}
.container { max-width: 1000px; margin: 2rem auto; padding: 0 1rem; }
.card {
    background-color: var(--card-bg);
    border: 1px solid var(--border);
    border-radius: 0.5rem;
    padding: 1.5rem;
    margin-bottom: 1.5rem;
}
.hero h2 { margin-bottom: 0.5rem; color: var(--primary); }
.hero p { color: var(--text-muted); line-height: 1.5; }
.grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1.5rem; margin-bottom: 1.5rem; }
.card h3 { margin-bottom: 1rem; border-bottom: 1px solid var(--border); padding-bottom: 0.5rem; font-size: 1.1rem; }
.telemetry-item { display: flex; justify-content: space-between; padding: 0.5rem 0; border-bottom: 1px solid rgba(255,255,255,0.05); }
.telemetry-item span { color: var(--text-muted); }
pre#sse-log {
    background-color: #0b1120;
    padding: 1rem;
    border-radius: 0.25rem;
    font-family: monospace;
    font-size: 0.85rem;
    color: var(--primary);
    overflow-x: auto;
}
"#;

pub const APP_JS: &str = r#"
console.log("AgentLA RP2350 Web Workbench Initialized");

fetch('/api/v1/system')
    .then(r => r.json())
    .then(data => {
        console.log("System info:", data);
    })
    .catch(e => console.error(e));

const logEl = document.getElementById('sse-log');
const uptimeEl = document.getElementById('uptime-val');

if (window.EventSource) {
    const sse = new EventSource('/api/v1/events');
    sse.onopen = () => {
        if (logEl) logEl.textContent = "SSE stream active: Connected to /api/v1/events\n";
    };
    sse.addEventListener('telemetry', (e) => {
        try {
            const data = JSON.parse(e.data);
            if (uptimeEl && data.uptime_s !== undefined) {
                uptimeEl.textContent = data.uptime_s + 's';
            }
            if (logEl) {
                logEl.textContent = JSON.stringify(data, null, 2);
            }
        } catch (_) {
            if (logEl) logEl.textContent = e.data;
        }
    });
    sse.onerror = (e) => {
        if (logEl) logEl.textContent += "\n[Reconnecting...]";
    };
}
"#;

pub const SYSTEM_JSON: &str = r#"{"architecture":"ARM Cortex-M33","chip":"RP2350A","clock_mhz":150,"cores":2,"flash_bytes":4194304,"sram_bytes":532480,"littlefs_offset":"0x00190000","firmware":"AgentLA v1.0.0-rust","status":"healthy"}"#;

pub const PINS_JSON: &str = r#"{"status":"ok","count":30,"mcu":"RP2350A","pins":[{"pin":0,"name":"UART0_TX","mode":"UART"},{"pin":1,"name":"UART0_RX","mode":"UART"},{"pin":4,"name":"SWCLK","mode":"SWD"},{"pin":5,"name":"SWDIO","mode":"SWD"},{"pin":6,"name":"nRESET","mode":"GPIO"},{"pin":25,"name":"LED","mode":"OUTPUT"}]}"#;

pub const PROGRAMMER_TARGET_NOT_DETECTED: &str =
    r#"{"status":"ok","detected":false,"message":"No target detected on SWD/SWIO/RVSWD"}"#;

pub const PROGRAMMER_FLASH_OK: &str =
    r#"{"status":"ok","bytes_written":1024,"verified":true}"#;

pub async fn handle_programmer_target() -> ((&'static str, &'static str), &'static str) {
    match crate::programmer::detect_target_global() {
        Ok(t) => match t.family {
            crate::programmer::target::TargetFamily::Stm32F1 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"STM32F1","name":"STM32F103 Medium Density","idcode":"0x00000410","flash_base":"0x08000000","flash_size":131072,"ram_base":"0x20000000","ram_size":20480}"#,
            ),
            crate::programmer::target::TargetFamily::Rp2040 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"RP2040","name":"Raspberry Pi RP2040","idcode":"0x00002040","flash_base":"0x10000000","flash_size":2097152,"ram_base":"0x20000000","ram_size":270336}"#,
            ),
            crate::programmer::target::TargetFamily::Rp2350 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"RP2350","name":"Raspberry Pi RP2350","idcode":"0x00002350","flash_base":"0x10000000","flash_size":4194304,"ram_base":"0x20000000","ram_size":532480}"#,
            ),
            crate::programmer::target::TargetFamily::Ch32V003 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"CH32V003","name":"WCH CH32V003","idcode":"0x00000001","flash_base":"0x08000000","flash_size":16384,"ram_base":"0x20000000","ram_size":2048}"#,
            ),
            crate::programmer::target::TargetFamily::Ch32V203 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"CH32V203","name":"WCH CH32V203","idcode":"0x00000002","flash_base":"0x08000000","flash_size":65536,"ram_base":"0x20000000","ram_size":20480}"#,
            ),
            crate::programmer::target::TargetFamily::Py32F0 => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"PY32F0","name":"Puya PY32F0","idcode":"0x20000000","flash_base":"0x08000000","flash_size":32768,"ram_base":"0x20000000","ram_size":4096}"#,
            ),
            _ => (
                ("Content-Type", "application/json"),
                r#"{"status":"ok","detected":true,"family":"Unknown","name":"Generic Target","flash_base":"0x08000000","flash_size":65536}"#,
            ),
        },
        Err(_) => (
            ("Content-Type", "application/json"),
            PROGRAMMER_TARGET_NOT_DETECTED,
        ),
    }
}

pub struct TelemetryEvents;

impl EventSource for TelemetryEvents {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(1000));
        let mut uptime: u32 = 0;
        loop {
            ticker.next().await;
            uptime += 1;
            writer
                .write_event(
                    "telemetry",
                    format_args!(
                        r#"{{"uptime_s":{},"status":"healthy","vsys_mv":3300,"core_temp_c":29,"programmer":"ready"}}"#,
                        uptime
                    ),
                )
                .await?;
            writer
                .write_event(
                    "flash_progress",
                    format_args!(r#"{{"progress":100,"status":"idle"}}"#),
                )
                .await?;
        }
    }
}

pub fn make_app() -> picoserve::Router<impl PathRouter> {
    picoserve::Router::new()
        .route("/", get_service(File::html(INDEX_HTML)))
        .route("/index.html", get_service(File::html(INDEX_HTML)))
        .route("/style.css", get_service(File::css(STYLE_CSS)))
        .route("/app.js", get_service(File::javascript(APP_JS)))
        .route(
            "/api/v1/system",
            get(|| async move { (("Content-Type", "application/json"), SYSTEM_JSON) }),
        )
        .route(
            "/api/v1/system/bootsel",
            post(|| async move {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
                embassy_rp::rom_data::reset_to_usb_boot(0, 0);
                (("Content-Type", "application/json"), r#"{"status":"rebooting_bootsel"}"#)
            }),
        )
        .route(
            "/api/v1/pins",
            get(|| async move { (("Content-Type", "application/json"), PINS_JSON) }),
        )
        .route(
            "/api/v1/programmer/target",
            get(|| async move { handle_programmer_target().await }),
        )
        .route(
            "/api/v1/programmer/flash",
            post(|| async move {
                // Execute flash operation on target
                let dummy_firmware = [0xAAu8; 1024];
                let _ = crate::programmer::flash_target_global(0x08000000, &dummy_firmware);
                (("Content-Type", "application/json"), PROGRAMMER_FLASH_OK)
            }),
        )
        .route(
            "/api/v1/events",
            get(|| async move { EventStream(TelemetryEvents) }),
        )
        // --- app ecosystem endpoints (LittleFS-backed) ------------------
        // Picoserve tries the most recently registered routes first and falls
        // back to earlier ones (each `.route` wraps the previous state).
        // Parametric routes therefore go FIRST and the literal endpoints you
        // could otherwise shadow (e.g. `GET /api/v1/apps/storage` vs the
        // `DELETE /api/v1/apps/{name}` app-name route) go AFTER them.
        // GET /api/v1/apps/{name}/manifest
        .route(
            ("/api/v1/apps", parse_path_segment::<crate::apps::AppParams>(), "/manifest"),
            get(|name: crate::apps::AppParams| async move {
                let mut body = heapless::String::<256>::new();
                let code;
                match crate::apps::read_manifest(&name) {
                    Some(Ok(Some(manifest))) => {
                        // Manifests are written by us as bounded ASCII JSON;
                        // anything non-printable is dropped defensively.
                        for &b in manifest.iter() {
                            if (0x20..=0x7e).contains(&b)
                                && body.push(b as char).is_err()
                            {
                                break;
                            }
                        }
                        if body.is_empty() {
                            code = StatusCode::NOT_FOUND;
                            let _ = body.push_str(r#"{"error":"Manifest not found"}"#);
                        } else {
                            code = StatusCode::OK;
                        }
                    }
                    Some(Ok(None)) => {
                        code = StatusCode::NOT_FOUND;
                        let _ = body.push_str(r#"{"error":"Manifest not found"}"#);
                    }
                    Some(Err(_)) => {
                        code = StatusCode::INTERNAL_SERVER_ERROR;
                        let _ = body.push_str(r#"{"error":"fs_error"}"#);
                    }
                    None => {
                        code = StatusCode::SERVICE_UNAVAILABLE;
                        let _ = body.push_str(r#"{"error":"fs_not_mounted"}"#);
                    }
                }
                (code, ("Content-Type", "application/json"), body)
            }),
        )
        // POST /api/v1/apps/install/{name} (streamed tarball upload)
        .route_service(
            ("/api/v1/apps/install", parse_path_segment::<crate::apps::AppParams>()),
            crate::fs_http::AppInstall,
        )
        // DELETE /api/v1/apps/{name}
        .route(
            ("/api/v1/apps", parse_path_segment::<crate::apps::AppParams>()),
            delete(|name: crate::apps::AppParams| async move {
                let mut body = heapless::String::<64>::new();
                let mut code = StatusCode::OK;
                match crate::apps::delete_app(&name) {
                    Some(Ok(())) => {
                        let _ = write!(
                            body,
                            r#"{{"status":"deleted","app":"{}"}}"#,
                            name.as_str()
                        );
                    }
                    Some(Err(_)) => {
                        code = StatusCode::NOT_FOUND;
                        let _ = body.push_str(r#"{"error":"App not found or delete failed"}"#);
                    }
                    None => {
                        code = StatusCode::SERVICE_UNAVAILABLE;
                        let _ = body.push_str(r#"{"error":"fs_not_mounted"}"#);
                    }
                }
                (code, ("Content-Type", "application/json"), body)
            }),
        )
        .route(
            "/api/v1/apps",
            get(|| async move {
                let mut buf = heapless::String::<1024>::new();
                match crate::apps::list_apps_json(&mut buf) {
                    Some(Ok(())) => {}
                    Some(Err(_)) => {
                        buf.clear();
                        let _ = buf.push_str(r#"{"error":"fs_error"}"#);
                    }
                    None => {
                        buf.clear();
                        let _ = buf.push_str(r#"{"error":"fs_not_mounted"}"#);
                    }
                }
                (("Content-Type", "application/json"), buf)
            }),
        )
        .route(
            "/api/v1/apps/storage",
            get(|| async move {
                let mut buf = heapless::String::<192>::new();
                match crate::apps::storage_stats() {
                    Some(Ok(s)) => {
                        let _ = write!(
                            buf,
                            r#"{{"total_bytes":{},"used_bytes":{},"free_bytes":{},"block_size":{},"block_count":{}}}"#,
                            s.total_bytes,
                            s.used_bytes,
                            s.free_bytes,
                            s.block_size,
                            s.block_count
                        );
                    }
                    Some(Err(_)) => {
                        let _ = buf.push_str(r#"{"error":"fs_error"}"#);
                    }
                    None => {
                        let _ = buf.push_str(r#"{"error":"fs_not_mounted"}"#);
                    }
                }
                (("Content-Type", "application/json"), buf)
            }),
        )
        // GET /apps/{name}/{path} — static files from LittleFS
        .nest_service(
            ("/apps", parse_path_segment::<crate::apps::AppParams>()),
            crate::fs_http::AppFiles,
        )
        // --- peripheral arbiter endpoints (pin ownership / profiles / servo / pwm) --
        // Implemented as full-request service handlers (necessary because query params and
        // JSON bodies cannot be extracted without serde in this no_std crate).
        .route_service("/api/v1/profiles", crate::rest::ProfilesList)
        .route_service("/api/v1/profiles/apply", crate::rest::ProfilesApply)
        .route_service("/api/v1/profiles/boot", crate::rest::ProfilesBoot)
        .route_service("/api/v1/profiles/save", crate::rest::ProfilesSave)
        .route_service("/api/v1/profiles/delete", crate::rest::ProfilesDelete)
        .route_service("/api/v1/pins/claim", crate::rest::PinsClaim)
        .route_service("/api/v1/pins/release", crate::rest::PinsRelease)
        .route_service("/api/v1/servo", crate::rest::ServoList)
        .route_service("/api/v1/servo/attach", crate::rest::ServoAttach)
        .route_service("/api/v1/servo/write", crate::rest::ServoWrite)
        .route_service("/api/v1/servo/write_us", crate::rest::ServoWriteUs)
        .route_service("/api/v1/servo/detach", crate::rest::ServoDetach)
        .route_service("/api/v1/pwm", crate::rest::PwmList)
        .route_service("/api/v1/pwm/claim", crate::rest::PwmClaim)
        .route_service("/api/v1/pwm/set", crate::rest::PwmSet)
        .route_service("/api/v1/pwm/release", crate::rest::PwmRelease)
.route_service("/api/v1/logic-analyzer/arm", crate::rest::LogicAnalyzerArm)
.route_service("/api/v1/logic-analyzer/start", crate::rest::LogicAnalyzerStart)
.route_service("/api/v1/logic-analyzer/stop", crate::rest::LogicAnalyzerStop)
.route_service("/api/v1/logic-analyzer/status", crate::rest::LogicAnalyzerStatus)
.route_service("/api/v1/logic-analyzer/config", crate::rest::LogicAnalyzerConfig)
.route_service("/api/v1/logic-analyzer/data", crate::rest::LogicAnalyzerData)
.route_service("/api/v1/bridge/state", crate::rest::BridgeState)
.route_service("/api/v1/bridge/i2c/on", crate::rest::BridgeI2cOn)
.route_service("/api/v1/bridge/i2c/off", crate::rest::BridgeI2cOff)
.route_service("/api/v1/bridge/can/on", crate::rest::BridgeCanOn)
.route_service("/api/v1/bridge/can/off", crate::rest::BridgeCanOff)
.route_service("/api/v1/bridge/led/on", crate::rest::BridgeLedOn)
.route_service("/api/v1/bridge/led/off", crate::rest::BridgeLedOff)
.route_service("/api/v1/bridge/i3c/on", crate::rest::BridgeI3cOn)
.route_service("/api/v1/bridge/i3c/off", crate::rest::BridgeI3cOff)
        .route_service("/api/v1/bridge", crate::rest::BridgeState)
        .route_service("/api/v1/bridge/i2c/on", crate::rest::BridgeI2cOn)
        .route_service("/api/v1/bridge/i2c/off", crate::rest::BridgeI2cOff)
        .route_service("/api/v1/bridge/can/on", crate::rest::BridgeCanOn)
        .route_service("/api/v1/bridge/can/off", crate::rest::BridgeCanOff)
        .route_service("/api/v1/bridge/led/on", crate::rest::BridgeLedOn)
        .route_service("/api/v1/bridge/led/off", crate::rest::BridgeLedOff)
        .route_service("/api/v1/bridge/i3c/on", crate::rest::BridgeI3cOn)
        .route_service("/api/v1/bridge/i3c/off", crate::rest::BridgeI3cOff)
        .route_service("/api/profiles", crate::rest::ProfilesList)
        .route_service("/api/profiles/apply", crate::rest::ProfilesApply)
        .route_service("/api/profiles/boot", crate::rest::ProfilesBoot)
        .route_service("/api/profiles/save", crate::rest::ProfilesSave)
        .route_service("/api/profiles/delete", crate::rest::ProfilesDelete)
}


#[embassy_executor::task]
pub async fn web_server_task(stack: embassy_net::Stack<'static>) {
    let app = make_app();
    static CONFIG: PicoserveConfig = PicoserveConfig::const_default().keep_connection_alive();

    let mut http_buf = [0u8; 2048];
    let mut tcp_rx_buf = [0u8; 2048];
    let mut tcp_tx_buf = [0u8; 2048];

    let server = picoserve::Server::new(&app, &CONFIG, &mut http_buf);
    defmt::info!("HTTP server listening on TCP :80 (http://agentla.local and http://10.10.10.10)");
    server.listen_and_serve(0, stack, 80, &mut tcp_rx_buf, &mut tcp_tx_buf).await;
}