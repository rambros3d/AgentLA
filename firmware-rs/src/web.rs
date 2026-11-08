use core::fmt::Write as _;

use agentla_bsp::BoardProfile;
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

pub const SYSTEM_JSON: &str = r#"{"architecture":"ARM Cortex-M33","chip":"RP2350A","clock_mhz":150,"cores":2,"flash_bytes":4194304,"sram_bytes":532480,"littlefs_offset":"0x00270000","firmware":"AgentLA v1.0.0-rust","status":"healthy"}"#;

pub const PINS_JSON: &str = r#"{"status":"ok","count":30,"mcu":"RP2350A","pins":[{"pin":0,"name":"UART0_TX","mode":"UART"},{"pin":1,"name":"UART0_RX","mode":"UART"},{"pin":4,"name":"SWCLK","mode":"SWD"},{"pin":5,"name":"SWDIO","mode":"SWD"},{"pin":6,"name":"nRESET","mode":"GPIO"},{"pin":25,"name":"LED","mode":"OUTPUT"}]}"#;

pub const PROGRAMMER_TARGET_NOT_DETECTED: &str =
    r#"{"status":"ok","detected":false,"message":"No target detected on SWD/SWIO/RVSWD"}"#;

pub const PROGRAMMER_FLASH_OK: &str =
    r#"{"status":"ok","bytes_written":1024,"verified":true}"#;

#[allow(dead_code)]
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

pub struct LogicAnalyzerEvents;

impl EventSource for LogicAnalyzerEvents {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(100));
        loop {
            ticker.next().await;
            let st = crate::analyzer::get_status();
            let state_str = match st.state {
                crate::analyzer::State::Idle => "idle",
                crate::analyzer::State::Armed => "armed",
                crate::analyzer::State::Capturing => "capturing",
                crate::analyzer::State::Captured => "captured",
                crate::analyzer::State::Error => "error",
                crate::analyzer::State::Stopped => "stopped",
            };
            writer
                .write_event(
                    "analyzer",
                    format_args!(
                        r#"{{"state":"{}","freq":{},"pre_bytes":{},"post_bytes":{},"sample_count":{}}}"#,
                        state_str, st.freq, st.pre_bytes, st.post_bytes, st.sample_count
                    ),
                )
                .await?;
        }
    }
}

pub struct OscilloscopeEvents;

impl EventSource for OscilloscopeEvents {
    async fn write_events<W: picoserve::io::Write>(
        self,
        mut writer: EventWriter<'_, W>,
    ) -> Result<(), W::Error> {
        let mut ticker = embassy_time::Ticker::every(embassy_time::Duration::from_millis(50));
        loop {
            ticker.next().await;
            if let Ok(frame) = crate::instruments::oscilloscope::capture_frame() {
                let m = &frame.metrics;
                writer
                    .write_event(
                        "frame",
                        format_args!(
                            r#"{{"ch_mask":{},"sample_rate":{},"vpp":[{}.{:02},{}.{:02},{}.{:02},{}.{:02}],"vrms":[{}.{:02},{}.{:02},{}.{:02},{}.{:02}],"freq":[{},{},{},{}],"duty":[{},{},{},{}],"sample_count":{}}}"#,
                            frame.ch_mask,
                            frame.sample_rate,
                            m[0].vpp_mv / 1000, (m[0].vpp_mv % 1000) / 10,
                            m[1].vpp_mv / 1000, (m[1].vpp_mv % 1000) / 10,
                            m[2].vpp_mv / 1000, (m[2].vpp_mv % 1000) / 10,
                            m[3].vpp_mv / 1000, (m[3].vpp_mv % 1000) / 10,
                            m[0].vrms_mv / 1000, (m[0].vrms_mv % 1000) / 10,
                            m[1].vrms_mv / 1000, (m[1].vrms_mv % 1000) / 10,
                            m[2].vrms_mv / 1000, (m[2].vrms_mv % 1000) / 10,
                            m[3].vrms_mv / 1000, (m[3].vrms_mv % 1000) / 10,
                            m[0].freq_hz, m[1].freq_hz, m[2].freq_hz, m[3].freq_hz,
                            m[0].duty_percent, m[1].duty_percent, m[2].duty_percent, m[3].duty_percent,
                            frame.sample_count
                        ),
                    )
                    .await?;
            }
        }
    }
}

pub fn make_app() -> picoserve::Router<impl PathRouter> {
    picoserve::Router::new()
        .route(
            "/",
            get(|| async move {
                let target = crate::rest::get_default_home_route();
                (
                    StatusCode::FOUND,
                    ("Location", target),
                    ("Cache-Control", "no-cache, no-store, must-revalidate"),
                    ("Content-Type", "text/plain"),
                    "Redirecting...",
                )
            }),
        )
        // --- Canonical System Apps & Instruments ------------------------
        .route("/home", get_service(crate::fs_http::SystemWebFile::new("index.html", "text/html")))
        .route("/home/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/home"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/scope", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        .route("/scope/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/scope"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/adapter", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        .route("/adapter/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/adapter"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/debug", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        .route("/debug/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/debug"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/pins", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        .route("/pins/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/pins"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/agent", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        .route("/agent/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/agent"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/terminal", get_service(crate::fs_http::SystemWebFile::new("terminal.html", "text/html")))
        .route("/terminal/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/terminal"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/terminal.html", get_service(crate::fs_http::SystemWebFile::new("terminal.html", "text/html")))
        .route("/files", get_service(crate::fs_http::SystemWebFile::new("files.html", "text/html")))
        .route("/files/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/files"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/files.html", get_service(crate::fs_http::SystemWebFile::new("files.html", "text/html")))
        .route("/index.html", get_service(crate::fs_http::SystemWebFile::new("index.html", "text/html")))
        .route("/lab.html", get_service(crate::fs_http::SystemWebFile::new("lab.html", "text/html")))
        // Legacy /apps and /lab aliases -> redirect to canonical routes
        .route("/apps", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/home"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/apps/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/home"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/apps.html", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/home"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/lab", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/scope"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/lab/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/scope"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        // In-ROM Device Diagnostics & Developer Console (Zero LittleFS dependency)
        .route("/device", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device.html", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/device/storage", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device/storage/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device/storage"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/device/pins", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device/pins/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device/pins"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/device/api", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device/api/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device/api"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/device/mcp", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/device/mcp/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device/mcp"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        .route("/diagnostics", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/diagnostics.html", get_service(File::html(crate::device_html::DEVICE_HTML)))
        .route("/diagnostics/", get(|| async move {
            (
                StatusCode::MOVED_PERMANENTLY,
                ("Location", "/device"),
                ("Cache-Control", "no-cache, no-store, must-revalidate"),
                ("Content-Type", "text/plain"),
                "Redirecting...",
            )
        }))
        // Subpath asset fallbacks (for browsers resolving relative to /home/ or /scope/)
        .route("/home/style.css", get_service(crate::fs_http::SystemWebFile::new("style.css", "text/css")))
        .route("/home/daisyui.css", get_service(crate::fs_http::SystemWebFile::new("daisyui.css", "text/css")))
        .route("/home/themes.css", get_service(crate::fs_http::SystemWebFile::new("themes.css", "text/css")))
        .route("/scope/style.css", get_service(crate::fs_http::SystemWebFile::new("style.css", "text/css")))
        .route("/scope/daisyui.css", get_service(crate::fs_http::SystemWebFile::new("daisyui.css", "text/css")))
        .route("/scope/themes.css", get_service(crate::fs_http::SystemWebFile::new("themes.css", "text/css")))
        .route("/scope/app.js", get_service(crate::fs_http::SystemWebFile::new("app.js", "application/javascript")))
        .route("/app.js", get_service(crate::fs_http::SystemWebFile::new("app.js", "application/javascript")))
        .route("/style.css", get_service(crate::fs_http::SystemWebFile::new("style.css", "text/css")))
        .route("/daisyui.css", get_service(crate::fs_http::SystemWebFile::new("daisyui.css", "text/css")))
        .route("/agentla-bridge.js", get_service(crate::fs_http::SystemWebFile::new("agentla-bridge.js", "application/javascript")))
        .route("/themes.css", get_service(crate::fs_http::SystemWebFile::new("themes.css", "text/css")))
        .route("/themes.json", get_service(crate::fs_http::SystemWebFile::new("themes.json", "application/json")))
        .route("/board.json", get_service(File::json(crate::board::ActiveBoard::manifest_json())))
        .route("/wallpaper.svg", get_service(crate::fs_http::SystemWebFile::new("wallpaper.svg", "image/svg+xml")))
        .route("/logo.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::LOGO_SVG)))
        .route("/favicon.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::FAVICON_SVG)))
        .route("/favicon.ico", get_service(File::from_content("image/svg+xml", crate::web_assets::FAVICON_SVG)))
        .route("/pinout.svg", get_service(File::from_content("image/svg+xml", crate::web_assets::PINOUT_SVG)))
        .route("/ok.json", get_service(crate::fs_http::SystemWebFile::new("ok.json", "application/json")))
        .route("/mcp.json", get_service(crate::fs_http::SystemWebFile::new("mcp.json", "application/json")))
        .route("/packages/catalog.json", get_service(crate::fs_http::SystemWebFile::new("packages/catalog.json", "application/json")))
        .route("/apps/catalog.json", get_service(crate::fs_http::SystemWebFile::new("apps/catalog.json", "application/json")))
        .route("/packages/servo-master.tar", get_service(crate::fs_http::SystemWebFile::new("packages/servo-master.tar", "application/x-tar")))
        .route("/packages/can-sniffer.tar", get_service(crate::fs_http::SystemWebFile::new("packages/can-sniffer.tar", "application/x-tar")))
        .route("/packages/neopixel-control.tar", get_service(crate::fs_http::SystemWebFile::new("packages/neopixel-control.tar", "application/x-tar")))
        .route("/skills/mcu-autonomous-testing/SKILL.md", get_service(crate::fs_http::SystemWebFile::new("skills/mcu-autonomous-testing/SKILL.md", "text/markdown; charset=utf-8")))
        .route_service("/api/v1/system/webui/update", crate::fs_http::SystemWebUiUpdate)
        .route_service("/api/v1/system/bootsel", crate::rest::SystemBootsel)
        .route_service("/api/v1/system/reboot", crate::rest::SystemReboot)
        .route_service("/api/v1/system/pins/release_all", crate::rest::SystemPinsReleaseAll)
        .route_service("/api/v1/system/pins/toggle", crate::rest::SystemPinsToggle)
        .route_service("/api/v1/system/fs/format", crate::rest::SystemFsFormat)
        .route_service("/api/v1/system", crate::rest::SystemInfoHandler)
        .route_service("/api/v1/system/clock", crate::rest::SystemClockHandler)
        .route_service("/api/v1/system/firmware/status", crate::rest::FirmwareStatus)
        .route_service("/api/v1/system/firmware/rollback", crate::rest::FirmwareRollback)
        .route_service("/api/v1/system/firmware/ota", crate::rest::FirmwareOtaStream)
        .route_service("/api/v1/system/telemetry", crate::rest::SystemTelemetry)
        .route_service("/api/v1/system/telemetry/stream", crate::rest::SystemTelemetryStream)
        .route_service("/api/v1/terminal/ws", crate::rest::TerminalWebSocket)
        .route_service("/api/v1/can/ws", crate::rest::CanWebSocket)
        .route_service("/api/v1/led/ws", crate::rest::LedWebSocket)
        .route(
            "/api/v1/pins",
            get(|| async move { (("Content-Type", "application/json"), PINS_JSON) }),
        )
        .route_service("/api/v1/programmer/target", crate::rest::ProgrammerTarget)
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
        // POST /api/v1/apps/install?name={name} (registered after parametric /api/v1/apps/{name} to avoid shadowing)
        .route(
            "/api/v1/apps/install",
            post_service(crate::fs_http::AppInstallQuery),
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
        // --- Filesystem REST API endpoints (system_fs & user_fs) ---
        .route_service("/api/v1/fs/stat", crate::fs_http::FsStat)
        .route_service("/api/v1/fs/list", crate::fs_http::FsList)
        .route_service("/api/v1/fs/read", crate::fs_http::FsRead)
        .route_service("/api/v1/fs/write", crate::fs_http::FsWrite)
        .route_service("/api/v1/fs/entry", crate::fs_http::FsEntry)
        .route_service("/api/v1/fs/mkdir", crate::fs_http::FsMkdir)
        .route_service("/api/v1/fs/rename", crate::fs_http::FsRename)

        .route_service("/mcp", crate::rest::McpRpc)
        .route_service("/mcp/health", crate::rest::McpHealth)
        .route_service("/api/v1/mcp", crate::rest::McpHealth)
        .route("/mcp/sse", get(|| async move { picoserve::response::sse::EventStream(crate::mcp::McpSseEvents) }))
        // GET /app/{name} -> redirect to /app/{name}/
        .route(
            ("/app", parse_path_segment::<crate::apps::AppParams>()),
            get(|name: crate::apps::AppParams| async move {
                let mut loc = heapless::String::<64>::new();
                let _ = write!(loc, "/app/{}/", name.as_str());
                (
                    StatusCode::MOVED_PERMANENTLY,
                    ("Location", loc),
                    ("Content-Type", "text/plain"),
                    "Redirecting...",
                )
            }),
        )
        // GET /app/{name}/{path} — static files from LittleFS
        .nest_service(
            ("/app", parse_path_segment::<crate::apps::AppParams>()),
            crate::fs_http::AppFiles,
        )
        // Backwards-compatible /apps/{name} -> redirect to /app/{name}/
        .route(
            ("/apps", parse_path_segment::<crate::apps::AppParams>()),
            get(|name: crate::apps::AppParams| async move {
                let mut loc = heapless::String::<64>::new();
                let _ = write!(loc, "/app/{}/", name.as_str());
                (
                    StatusCode::MOVED_PERMANENTLY,
                    ("Location", loc),
                    ("Content-Type", "text/plain"),
                    "Redirecting...",
                )
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
        .route("/api/board/manifest", get_service(File::json(crate::board::ActiveBoard::manifest_json())))
        .route_service("/api/v1/themes", crate::rest::ThemesEndpoint)
        .route_service("/api/v1/settings", crate::rest::SettingsEndpoint)
        .route_service("/api/settings", crate::rest::SettingsEndpoint)
        .route_service("/api/v1/programmer/reset", crate::rest::ProgrammerReset)
        .route_service("/api/v1/programmer/reset-catch", crate::rest::ProgrammerResetCatch)
        .route_service("/api/v1/programmer/rescue/stm32", crate::rest::ProgrammerRescueStm32)
        .route_service("/api/v1/programmer/rescue/nrf", crate::rest::ProgrammerRescueNrf)
        .route_service("/api/v1/programmer/erase", crate::rest::ProgrammerErase)
        .route_service("/api/v1/programmer/read", crate::rest::ProgrammerRead)
        .route_service("/api/v1/programmer/rtt/status", crate::rest::ProgrammerRttStatus)
        .route_service("/api/v1/programmer/rtt/stream", crate::rest::ProgrammerRttStream)
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
        .route("/api/v1/logic-analyzer/stream", get(|| async move { picoserve::response::sse::EventStream(LogicAnalyzerEvents) }))
        .route_service("/api/v1/analyzer/stream", crate::rest::AnalyzerStream)
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
        .route_service("/api/v1/bridge/can", crate::rest::BridgeCan)
        .route_service("/api/v1/bridge/can/termination", crate::rest::BridgeCanTermination)
        .route_service("/api/v1/bridge/can/on", crate::rest::BridgeCanOn)
        .route_service("/api/v1/bridge/can/off", crate::rest::BridgeCanOff)
        .route_service("/api/v1/bridge/led/on", crate::rest::BridgeLedOn)
        .route_service("/api/v1/bridge/led/off", crate::rest::BridgeLedOff)
        .route_service("/api/v1/bridge/led/transmit", crate::rest::BridgeLedTransmit)
        .route_service("/api/v1/bus/led/transmit", crate::rest::BridgeLedTransmit)
        .route_service("/api/v1/bridge/i3c/on", crate::rest::BridgeI3cOn)
        .route_service("/api/v1/bridge/i3c/off", crate::rest::BridgeI3cOff)
        .route_service("/api/v1/bridge/spi/on", crate::rest::BridgeSpiOn)
        .route_service("/api/v1/bridge/spi/off", crate::rest::BridgeSpiOff)
        .route_service("/api/v1/bridge/spi/transfer", crate::rest::BridgeSpiTransfer)
        .route_service("/api/v1/bus/spi/transfer", crate::rest::BridgeSpiTransfer)
        .route_service("/api/v1/bridge/uart/on", crate::rest::BridgeUartOn)
        .route_service("/api/v1/bridge/uart/off", crate::rest::BridgeUartOff)
        .route_service("/api/v1/bridge/uart/write", crate::rest::BridgeUartWrite)
        .route_service("/api/v1/bus/uart/write", crate::rest::BridgeUartWrite)
        .route_service("/api/v1/bridge/uart/read", crate::rest::BridgeUartRead)
        .route_service("/api/v1/bus/uart/read", crate::rest::BridgeUartRead)
        .route_service("/api/profiles", crate::rest::ProfilesList)
        .route_service("/api/profiles/apply", crate::rest::ProfilesApply)
        .route_service("/api/profiles/boot", crate::rest::ProfilesBoot)
        .route_service("/api/profiles/save", crate::rest::ProfilesSave)
        .route_service("/api/profiles/delete", crate::rest::ProfilesDelete)
        // --- Mixed-Signal Instrument Suite endpoints ---
        .route_service("/api/v1/oscilloscope/data", crate::rest::ScopeData)
        .route_service("/api/v1/oscilloscope/start", crate::rest::ScopeStart)
        .route_service("/api/v1/oscilloscope/stop", crate::rest::ScopeStop)
        .route_service("/api/v1/scope/data", crate::rest::ScopeData)
        .route_service("/api/v1/scope/start", crate::rest::ScopeStart)
        .route_service("/api/v1/scope/stop", crate::rest::ScopeStop)
        .route("/api/v1/oscilloscope/stream", get(|| async move { picoserve::response::sse::EventStream(OscilloscopeEvents) }))
        .route("/api/v1/scope/stream", get(|| async move { picoserve::response::sse::EventStream(OscilloscopeEvents) }))
        .route_service("/api/v1/generator/start", crate::rest::GeneratorStart)
        .route_service("/api/v1/generator/stop", crate::rest::GeneratorStop)
        .route_service("/api/v1/generator/status", crate::rest::GeneratorStatus)
        .route_service("/api/v1/led/pattern", crate::rest::LedPattern)
        .route_service("/api/v1/led/analyze", crate::rest::LedAnalyze)
        .route_service("/api/v1/glitch/arm", crate::rest::GlitchArm)
        .route_service("/api/v1/glitch/disarm", crate::rest::GlitchDisarm)
        .route_service("/api/v1/glitch/status", crate::rest::GlitchStatus)
}


#[embassy_executor::task]
pub async fn web_server_task(stack: embassy_net::Stack<'static>) {
    let app = make_app();
    static CONFIG: PicoserveConfig = PicoserveConfig::const_default().keep_connection_alive();

    let mut http_buf_0 = [0u8; 2048];
    let mut tcp_rx_buf_0 = [0u8; 1536];
    let mut tcp_tx_buf_0 = [0u8; 1536];
    let server_0 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_0);

    let mut http_buf_1 = [0u8; 2048];
    let mut tcp_rx_buf_1 = [0u8; 1536];
    let mut tcp_tx_buf_1 = [0u8; 1536];
    let server_1 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_1);

    let mut http_buf_2 = [0u8; 2048];
    let mut tcp_rx_buf_2 = [0u8; 1536];
    let mut tcp_tx_buf_2 = [0u8; 1536];
    let server_2 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_2);

    let mut http_buf_3 = [0u8; 2048];
    let mut tcp_rx_buf_3 = [0u8; 1536];
    let mut tcp_tx_buf_3 = [0u8; 1536];
    let server_3 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_3);

    let mut http_buf_4 = [0u8; 2048];
    let mut tcp_rx_buf_4 = [0u8; 1536];
    let mut tcp_tx_buf_4 = [0u8; 1536];
    let server_4 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_4);

    let mut http_buf_5 = [0u8; 2048];
    let mut tcp_rx_buf_5 = [0u8; 1536];
    let mut tcp_tx_buf_5 = [0u8; 1536];
    let server_5 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_5);

    let mut http_buf_6 = [0u8; 2048];
    let mut tcp_rx_buf_6 = [0u8; 1536];
    let mut tcp_tx_buf_6 = [0u8; 1536];
    let server_6 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_6);

    let mut http_buf_7 = [0u8; 2048];
    let mut tcp_rx_buf_7 = [0u8; 1536];
    let mut tcp_tx_buf_7 = [0u8; 1536];
    let server_7 = picoserve::Server::new(&app, &CONFIG, &mut http_buf_7);

    defmt::info!("HTTP server listening on TCP :80 with 8 concurrent worker sockets");
    embassy_futures::join::join(
        embassy_futures::join::join4(
            server_0.listen_and_serve(0, stack, 80, &mut tcp_rx_buf_0, &mut tcp_tx_buf_0),
            server_1.listen_and_serve(1, stack, 80, &mut tcp_rx_buf_1, &mut tcp_tx_buf_1),
            server_2.listen_and_serve(2, stack, 80, &mut tcp_rx_buf_2, &mut tcp_tx_buf_2),
            server_3.listen_and_serve(3, stack, 80, &mut tcp_rx_buf_3, &mut tcp_tx_buf_3),
        ),
        embassy_futures::join::join4(
            server_4.listen_and_serve(4, stack, 80, &mut tcp_rx_buf_4, &mut tcp_tx_buf_4),
            server_5.listen_and_serve(5, stack, 80, &mut tcp_rx_buf_5, &mut tcp_tx_buf_5),
            server_6.listen_and_serve(6, stack, 80, &mut tcp_rx_buf_6, &mut tcp_tx_buf_6),
            server_7.listen_and_serve(7, stack, 80, &mut tcp_rx_buf_7, &mut tcp_tx_buf_7),
        ),
    ).await;
}