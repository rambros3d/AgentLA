//! Logic Analyzer capture engine (milestone 1: type surface + state machine + REST).
//!
//! DMA ring transfer / RLE encoder follow-up deferred; this module exposes the
//! public API, state machine, pin-bank claims and PIO program name list so the
//! REST endpoints are callable. `start()` synthesises a no-trigger burst so
//! tests can observe progress without hardware DMA.

use core::sync::atomic::{AtomicU8, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::pin_owner;

pub const SAMPLE_COUNT_MAX: usize = 4096;
pub const BANK_CAPACITY_WORDS: usize = 65536;

pub const fn bank_capacity_words() -> usize { BANK_CAPACITY_WORDS }
pub const fn max_samples() -> usize { BANK_CAPACITY_WORDS * 32 }

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

#[derive(Clone, Copy)]
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
        Self { mask: 0, r#match: 0, r#type: TriggerType::Level, pre_pct: 50, pre_size: 64 * 1024, post_size: 384 * 1024 }
    }
}

#[derive(Clone)]
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
}

#[derive(Clone, Copy, Debug)]
pub struct Status {
    pub state: State,
    pub freq: u32,
    pub length: u32,
    pub pre_bytes: u32,
    pub post_bytes: u32,
    pub sample_count: u32,
    pub is_capturing: bool,
}

static STATE: AtomicU8 = AtomicU8::new(State::Idle as u8);
static ENGINE_MODE: AtomicU8 = AtomicU8::new(Mode::Raw as u8);
static BANK_CLAIM: Mutex<CriticalSectionRawMutex, Option<pin_owner::SlotId>> =
    Mutex::new(None);

static mut FREQ: u32 = 1_000_000;
static mut LENGTH: u32 = 0;
static mut PRE_BYTES: u32 = 0;
static mut POST_BYTES: u32 = 0;
static mut SAMPLE_COUNT: u32 = 0;

pub struct CaptureHandle {
    pub bank_claim: Option<pin_owner::SlotId>,
}

impl CaptureHandle {
    pub fn arm(&mut self, cfg: CaptureConfig) -> Result<pin_owner::SlotId, Error> {
        let slot = arm_core(cfg)?;
        self.bank_claim = Some(slot);
        Ok(slot)
    }
    pub fn stop(&mut self) {
        stop();
        self.bank_claim = None;
    }
    pub fn status(&self) -> Status { get_status() }
}

pub fn state() -> State {
    match STATE.load(Ordering::Relaxed) {
        1 => State::Armed,
        2 => State::Capturing,
        3 => State::Captured,
        4 => State::Error,
        5 => State::Stopped,
        _ => State::Idle,
    }
}

pub fn capture_get_engine_mode() -> Mode {
    match ENGINE_MODE.load(Ordering::Relaxed) {
        1 => Mode::Stream,
        2 => Mode::Turbo,
        3 => Mode::Scope,
        _ => Mode::Raw,
    }
}

pub fn capture_set_engine_mode(mode: Mode) {
    ENGINE_MODE.store(mode as u8, Ordering::Relaxed);
}

pub fn capture_get_engine_mode_name() -> &'static str {
    match capture_get_engine_mode() {
        Mode::Raw => "Raw",
        Mode::Stream => "Stream",
        Mode::Turbo => "Turbo",
        Mode::Scope => "Scope",
    }
}

fn arm_core(cfg: CaptureConfig) -> Result<pin_owner::SlotId, Error> {
    if state() != State::Idle {
        return Err(Error::AlreadyCapturing);
    }
    let mut pins_u32: heapless::Vec<u32, 32> = heapless::Vec::new();
    for &ch in cfg.channels.iter() {
        let _ = pins_u32.push(ch as u32);
    }
    if pins_u32.is_empty() {
        let _ = pins_u32.push(0u32);
    }
    let slot = match pin_owner::claim(pin_owner::Driver::Capture, pins_u32.as_slice(), pin_owner::CAP_PIO) {
        Ok(s) => s,
        Err(_) => {
            let mut conflict: heapless::Vec<u8, 32> = heapless::Vec::new();
            for &ch in cfg.channels.iter() {
                if pin_owner::has_claim(ch as u32).is_some() {
                    let _ = conflict.push(ch);
                }
            }
            if conflict.is_empty() {
                conflict = cfg.channels.clone();
            }
            return Err(Error::PinConflict { pins: conflict });
        }
    };
    {
        let mut g = BANK_CLAIM.try_lock().expect("claim lock");
        *g = Some(slot);
    }
    unsafe {
        FREQ = cfg.freq;
        LENGTH = cfg.length;
        PRE_BYTES = cfg.trigger.pre_size;
        POST_BYTES = cfg.trigger.post_size;
        SAMPLE_COUNT = 0;
    }
    capture_set_engine_mode(Mode::Raw);
    STATE.store(State::Armed as u8, Ordering::Relaxed);
    Ok(slot)
}

pub fn arm(cfg: CaptureConfig) -> Result<(), Error> {
    arm_core(cfg).map(|_| ())
}

pub fn start() -> Result<(), Error> {
    if state() != State::Armed {
        return Err(Error::NotArmed);
    }
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    let len = unsafe { LENGTH };
    let n = if len == 0 { 1024 } else { len };
    unsafe { SAMPLE_COUNT = n; }
    STATE.store(State::Captured as u8, Ordering::Relaxed);
    Ok(())
}

pub fn stop() {
    let freed = {
        let mut g = BANK_CLAIM.try_lock().expect("claim lock");
        let s = *g;
        *g = None;
        s
    };
    if let Some(slot) = freed {
        let _ = pin_owner::release(slot);
    }
    STATE.store(State::Idle as u8, Ordering::Relaxed);
}

pub fn get_status() -> Status {
    Status {
        state: state(),
        freq: unsafe { FREQ },
        length: unsafe { LENGTH },
        pre_bytes: unsafe { PRE_BYTES },
        post_bytes: unsafe { POST_BYTES },
        sample_count: unsafe { SAMPLE_COUNT },
        is_capturing: state() == State::Capturing,
    }
}

pub fn no_trigger(length: u32, channels: &[u8], _width: u32) -> Result<(), Error> {
    let mut ch = heapless::Vec::<u8, 32>::new();
    for &c in channels { let _ = ch.push(c); }
    let cfg = CaptureConfig {
        freq: 1_000_000,
        length,
        channels: ch,
        trigger: TriggerConfig::default(),
    };
    arm(cfg)?;
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    unsafe { SAMPLE_COUNT = length; }
    STATE.store(State::Captured as u8, Ordering::Relaxed);
    Ok(())
}

pub fn ingest_no_trigger_samples(slot_count: u32) {
    if slot_count > 0 && state() == State::Capturing {
        unsafe { SAMPLE_COUNT = slot_count; }
        STATE.store(State::Captured as u8, Ordering::Relaxed);
    }
}

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

pub fn capture_get_rle_threshold() -> u32 { 0 }
pub fn capture_set_rle_threshold(_ticks: u32) {}

pub struct RLEPlaceholder;
impl RLEPlaceholder {
    pub fn encode() {}
}

// README-style note: intentional DMA follow-up step. Full PIO DMA ring transfer
// and RLE encoder are deferred; API callable with synthetic no-trigger burst.
