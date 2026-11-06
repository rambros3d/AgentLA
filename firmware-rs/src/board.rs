//! Active Board Profile selection and re-exports.

pub use agentla_bsp::*;

#[cfg(not(feature = "board-bakuprobe"))]
pub use agentla_board_pico2::Pico2Board as ActiveBoard;

#[cfg(feature = "board-bakuprobe")]
pub use agentla_board_bakuprobe::BakuProbeBoard as ActiveBoard;
