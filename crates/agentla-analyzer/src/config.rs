//! Capture configuration, states, and error types.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle = 0,
    Armed = 1,
    Capturing = 2,
    Captured = 3,
    Error = 4,
    Stopped = 5,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Raw = 0,
    Stream = 1,
    Turbo = 2,
    Scope = 3,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriggerType {
    Level = 0,
    RisingEdge = 1,
    FallingEdge = 2,
    Edge = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TriggerConfig {
    pub mask: u32,
    pub r#match: u32,
    pub r#type: TriggerType,
    pub pre_pct: u8,
    pub pre_size: u32,
    pub post_size: u32,
}

impl TriggerConfig {
    pub const fn default() -> Self {
        Self {
            mask: 0,
            r#match: 0,
            r#type: TriggerType::Level,
            pre_pct: 50,
            pre_size: 64 * 1024,
            post_size: 384 * 1024,
        }
    }
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureConfig {
    pub freq: u32,
    pub length: u32,
    pub channels: heapless::Vec<u8, 32>,
    pub trigger: TriggerConfig,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Error {
    AlreadyCapturing,
    PinConflict { pins: heapless::Vec<u8, 32> },
    NotArmed,
    ClaimFailed,
    OutOfMemory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub state: State,
    pub freq: u32,
    pub length: u32,
    pub pre_bytes: u32,
    pub post_bytes: u32,
    pub sample_count: u32,
    pub is_capturing: bool,
}

pub const SAMPLE_COUNT_MAX: usize = 1024;
pub const BANK_WORDS: usize = 1024;

pub fn program_names() -> &'static [&'static str] {
    &[
        "BLAST_CAPTURE",
        "POSITIVE_CAPTURE",
        "NEGATIVE_CAPTURE",
        "COMPLEX_CAPTURE",
        "FAST_CAPTURE",
        "NO_TRIGGER_CAPTURE",
        "pio_rle_capture",
        "pio_trigger_qualifier",
    ]
}

pub fn capture_get_engine_mode_name() -> &'static str {
    "Raw"
}
