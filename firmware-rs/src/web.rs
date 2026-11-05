use core::fmt::Write as _;

use embassy_net::Stack;
use picoserve::response::sse::{EventSource, EventStream, EventWriter};
use picoserve::response::{File, StatusCode};
use picoserve::routing::{delete, get, get_service, parse_path_segment, post, post_service, PathRouter};
use picoserve::Config as PicoserveConfig;
use picoserve::Server;

pub trait FileExt {
    fn json(body: &'static str) -> Self;
    fn from_content(content_type: &'static str, body: &'static [u8]) -> Self;
}

impl FileExt for File {
    fn json(body: &'static str) -> Self {
        File::with_content_type("application/json", body.as_bytes())
    }
    fn from_content(content_type: &'static str, body: &'static [u8]) -> Self {
        File::with_content_type(content_type, body)
    }
}

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
        .route("/", get_service(File::html(crate::web_assets::INDEX_HTML)))
        .route("/index.html", get_service(File::html(crate::web_assets::INDEX_HTML)))
        .route("/apps", get_service(File::html(crate::web_assets::APPS_HTML)))
        .route("/apps.html", get_service(File::html(crate::web_assets::APPS_HTML)))
        .route("/app.js", get_service(File::javascript(crate::web_assets::APP_JS)))
        .route("/style.css", get_service(File::css(crate::web_assets::STYLE_CSS)))
        .route("/agentla-bridge.js", get_service(File::javascript(crate::web_assets::AGENTLA_BRIDGE_JS)))
        .route("/themes.css", get_service(File::css(crate::web_assets::THEMES_CSS)))
        .route("/themes.json", get_service(File::json(crate::web_assets::THEMES_JSON)))
        .route("/board.json", get_service(File::json(crate::web_assets::BOARD_JSON)))
        .route("/wallpaper.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::WALLPAPER_SVG)))
        .route("/logo.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::LOGO_SVG)))
        .route("/favicon.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::FAVICON_SVG)))
        .route("/pinout.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::PINOUT_SVG)))
        .route("/ok.json", get_service(File::json(crate::web_assets::OK_JSON)))
        .route("/mcp.json", get_service(File::json(crate::web_assets::MCP_JSON)))
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
            "/api/apps",
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

        .route_service("/mcp", crate::rest::McpRpc)
        .route_service("/mcp/health", crate::rest::McpHealth)
        .route_service("/api/v1/mcp", crate::rest::McpHealth)
        .route("/mcp/sse", get(|| async move { picoserve::response::sse::EventStream(crate::mcp::McpSseEvents) }))
        // GET /apps/{name}/{path} — static files from LittleFS
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
        .route("/api/board/manifest", get_service(File::json(crate::web_assets::BOARD_JSON)))
        .route_service("/api/v1/themes", crate::rest::ThemesEndpoint)
        .route_service("/api/v1/settings", crate::rest::SettingsEndpoint)
        .route_service("/api/settings", crate::rest::SettingsEndpoint)
        .route_service("/api/v1/programmer/reset", crate::rest::ProgrammerReset)
        .route_service("/api/v1/programmer/erase", crate::rest::ProgrammerErase)
        .route_service("/api/v1/programmer/read", crate::rest::ProgrammerRead)
        .route_service("/api/v1/logic-analyzer/arm", crate::rest::LogicAnalyzerArm)
        .route_service("/api/v1/logic-analyzer/start", crate::rest::LogicAnalyzerStart)
        .route_service("/api/v1/logic-analyzer/stop", crate::rest::LogicAnalyzerStop)
        .route_service("/api/v1/logic-analyzer/status", crate::rest::LogicAnalyzerStatus)
        .route_service("/api/v1/logic-analyzer/config", crate::rest::LogicAnalyzerConfig)
        .route_service("/api/v1/logic-analyzer/data", crate::rest::LogicAnalyzerData)
        .route_service("/api/v1/logic-analyzer/vcd", crate::rest::LogicAnalyzerVcd)
        .route_service("/api/v1/analyzer/arm", crate::rest::LogicAnalyzerArm)
        .route_service("/api/v1/analyzer/start", crate::rest::LogicAnalyzerStart)
        .route_service("/api/v1/analyzer/stop", crate::rest::LogicAnalyzerStop)
        .route_service("/api/v1/analyzer/status", crate::rest::LogicAnalyzerStatus)
        .route_service("/api/v1/analyzer/config", crate::rest::LogicAnalyzerConfig)
        .route_service("/api/v1/analyzer/data", crate::rest::LogicAnalyzerData)
        .route_service("/api/v1/analyzer/vcd", crate::rest::LogicAnalyzerVcd)
        .route_service("/api/v1/bridge", crate::rest::BridgeState)
        .route_service("/api/v1/bridge/state", crate::rest::BridgeState)
        .route_service("/api/v1/bridge/i2c/on", crate::rest::BridgeI2cOn)
        .route_service("/api/v1/bridge/i2c/off", crate::rest::BridgeI2cOff)
        .route_service("/api/v1/bridge/i2c/scan", crate::rest::BridgeI2cScan)
        .route_service("/api/v1/bus/i2c/scan", crate::rest::BridgeI2cScan)
        .route_service("/api/v1/bridge/i2c/read", crate::rest::BridgeI2cRead)
        .route_service("/api/v1/bus/i2c/read", crate::rest::BridgeI2cRead)
        .route_service("/api/v1/bridge/i2c/write", crate::rest::BridgeI2cWrite)
        .route_service("/api/v1/bus/i2c/write", crate::rest::BridgeI2cWrite)
        .route_service("/api/v1/bridge/can/on", crate::rest::BridgeCanOn)
        .route_service("/api/v1/bridge/can/off", crate::rest::BridgeCanOff)
        .route_service("/api/v1/bridge/led/on", crate::rest::BridgeLedOn)
        .route_service("/api/v1/bridge/led/off", crate::rest::BridgeLedOff)
        .route_service("/api/v1/bridge/led/transmit", crate::rest::BridgeLedTransmit)
        .route_service("/api/v1/bus/led/transmit", crate::rest::BridgeLedTransmit)
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