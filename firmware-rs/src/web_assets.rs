//! Static board assets for the AgentLA interface.
//!
//! WebUI HTML/CSS/JS assets are decoupled from firmware ROM and dynamically
//! streamed from LittleFS Partition 1 (`SYSTEM_FS`). Board-specific vector SVGs
//! and hardware manifests remain available here.

#[cfg(not(feature = "board-bakuprobe"))]
pub use agentla_board_pico2::{FAVICON_SVG, LOGO_SVG, MANIFEST_JSON as BOARD_JSON, PINOUT_SVG};

#[cfg(feature = "board-bakuprobe")]
pub use agentla_board_bakuprobe::{FAVICON_SVG, LOGO_SVG, MANIFEST_JSON as BOARD_JSON, PINOUT_SVG};
