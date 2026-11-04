use embassy_net::Stack;
use picoserve::response::sse::{EventSource, EventStream, EventWriter};
use picoserve::response::File;
use picoserve::routing::{get, get_service, PathRouter};
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
                        r#"{{"uptime_s":{},"status":"healthy","vsys_mv":3300,"core_temp_c":29}}"#,
                        uptime
                    ),
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
            "/api/v1/pins",
            get(|| async move { (("Content-Type", "application/json"), PINS_JSON) }),
        )
        .route(
            "/api/v1/events",
            get(|| async move { EventStream(TelemetryEvents) }),
        )
}

#[embassy_executor::task]
pub async fn web_server_task(stack: Stack<'static>) {
    let app = make_app();
    static CONFIG: PicoserveConfig = PicoserveConfig::const_default().keep_connection_alive();

    let mut http_buf = [0u8; 2048];
    let mut tcp_rx_buf = [0u8; 2048];
    let mut tcp_tx_buf = [0u8; 2048];

    let server = Server::new(&app, &CONFIG, &mut http_buf);
    defmt::info!("HTTP server listening on TCP :80 (http://agentla.local and http://10.10.10.10)");
    server
        .listen_and_serve(0, stack, 80, &mut tcp_rx_buf, &mut tcp_tx_buf)
        .await;
}
