//! Logic Analyzer capture engine (MVP scaffold).
//!
//! This module provides the capture state machine that future phases (PIO DMA programs,
//! RLE encoder, and the 11 decoders) will drive. The MVP keeps the surface small so the
//! rest of the firmware can link and the REST endpoints can report state, allowing
//! iterative development without blocking the async runtime.

use core::sync::atomic::{AtomicU8, Ordering};

pub const SAMPLE_COUNT_MAX: usize = 4096;
pub const BANK_CAPACITY_WORDS: usize = 16384;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum State {
    Idle = 0,
    Armed = 1,
    Capturing = 2,
    Captured = 3,
    Error = 4,
    Stopped = 5,
}

pub struct Config {
    pub freq_hz: u32,
    pub length: u32,
    pub pre_pct: u8,
}

impl Config {
    pub const fn default() -> Self { Self { freq_hz: 1_000_000, length: 1024, pre_pct: 50 } }
}

static STATE: AtomicU8 = AtomicU8::new(State::Idle as u8);

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

pub fn arm(cfg: Config) -> Result<(), ()> {
    let _ = cfg;
    if state() != State::Idle && state() != State::Captured && state() != State::Stopped {
        return Err(());
    }
    STATE.store(State::Armed as u8, Ordering::Relaxed);
    Ok(())
}

pub fn start() -> Result<(), ()> {
    if state() != State::Armed { return Err(()); }
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    Ok(())
}

pub fn stop() {
    STATE.store(State::Stopped as u8, Ordering::Relaxed);
}

pub fn last_error() -> Option<&'static str> { None }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriggerMode { None, RisingEdge, FallingEdge, Pattern }

pub struct CaptureBank(pub [u8; SAMPLE_COUNT_MAX*4]);

#[allow(static_mut_refs)]
static mut BANK: CaptureBank = CaptureBank([0u8; SAMPLE_COUNT_MAX*4]);



// Additional state-transition helper (no-op except state the intended bank size)
pub fn ingest_no_trigger_samples(slot_count: u32) {
    if slot_count > 0 && state() == State::Capturing {
        STATE.store(State::Captured as u8, core::sync::atomic::Ordering::Relaxed);
    }
}
