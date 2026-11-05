//! Embedded static web assets for the AgentLA Web Desktop interface.

pub const INDEX_HTML: &str = include_str!("../../firmware/html/index.html");
pub const APPS_HTML: &str = include_str!("../../firmware/html/apps.html");
pub const APP_JS: &str = include_str!("../../firmware/html/app.js");
pub const STYLE_CSS: &str = include_str!("../../firmware/html/style.css");
pub const AGENTLA_BRIDGE_JS: &str = include_str!("../../firmware/html/agentla-bridge.js");
pub const THEMES_CSS: &str = include_str!("../../firmware/html/themes.css");
pub const THEMES_JSON: &str = include_str!("../../firmware/html/themes.json");
pub const BOARD_JSON: &str = include_str!("../../boards/pico2/board.json");
pub const WALLPAPER_SVG: &[u8] = include_bytes!("../../firmware/html/wallpaper.svg");
pub const LOGO_SVG: &[u8] = include_bytes!("../../boards/pico2/logo.svg");
pub const FAVICON_SVG: &[u8] = include_bytes!("../../boards/pico2/favicon.svg");
pub const PINOUT_SVG: &[u8] = include_bytes!("../../boards/pico2/pinout.svg");
pub const OK_JSON: &str = include_str!("../../firmware/html/ok.json");
pub const MCP_JSON: &str = include_str!("../../firmware/html/mcp.json");
