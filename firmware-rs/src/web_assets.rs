//! Embedded static web assets for the AgentLA Web Desktop interface.

#[cfg(not(feature = "board-bakuprobe"))]
pub use agentla_board_pico2::{FAVICON_SVG, LOGO_SVG, MANIFEST_JSON as BOARD_JSON, PINOUT_SVG};

#[cfg(feature = "board-bakuprobe")]
pub use agentla_board_bakuprobe::{FAVICON_SVG, LOGO_SVG, MANIFEST_JSON as BOARD_JSON, PINOUT_SVG};

pub const INDEX_HTML: &str = include_str!("../../firmware/html/index.html");
pub const APPS_HTML: &str = include_str!("../../firmware/html/apps.html");
pub const APP_JS: &str = include_str!("../../firmware/html/app.js");
pub const STYLE_CSS: &str = include_str!("../../firmware/html/style.css");
pub const AGENTLA_BRIDGE_JS: &str = include_str!("../../firmware/html/agentla-bridge.js");
pub const THEMES_CSS: &str = include_str!("../../firmware/html/themes.css");
pub const THEMES_JSON: &str = include_str!("../../firmware/html/themes.json");
pub const WALLPAPER_SVG: &[u8] = include_bytes!("../../firmware/html/wallpaper.svg");
pub const OK_JSON: &str = include_str!("../../firmware/html/ok.json");
pub const MCP_JSON: &str = include_str!("../../firmware/html/mcp.json");
