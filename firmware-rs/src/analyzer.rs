//! Logic analyzer core + compression utilities (Rust port of C firmware).

use core::sync::atomic::{AtomicU8, Ordering};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::pin_owner;
use rp_pac as pac;

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
pub enum Mode { Raw = 0, Stream = 1, Turbo = 2, Scope = 3 }

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TriggerType { Level = 0, RisingEdge = 1, FallingEdge = 2, Edge = 3 }

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
pub enum Error { AlreadyCapturing, PinConflict { pins: heapless::Vec<u8, 32> }, NotArmed, ClaimFailed, OutOfMemory }

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

pub const SAMPLE_COUNT_MAX: usize = 1024;

static STATE: AtomicU8 = AtomicU8::new(State::Idle as u8);
static BANK_CLAIM: Mutex<CriticalSectionRawMutex, Option<pin_owner::SlotId>> = Mutex::new(None);

static mut FREQ: u32 = 1_000_000;
static mut LENGTH: u32 = 0;
static mut PRE_BYTES: u32 = 0;
static mut POST_BYTES: u32 = 0;
static mut SAMPLE_COUNT: u32 = 0;
static mut TRIG_PIN: u8 = 0;
static TRIGGER_MODE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
pub const BANK_WORDS: usize = 1024;
static mut SAMPLE_RING: [u32; BANK_WORDS] = [0; BANK_WORDS];

pub fn state() -> State {
    match STATE.load(Ordering::Relaxed) {
        1 => State::Armed, 2 => State::Capturing, 3 => State::Captured,
        4 => State::Error, 5 => State::Stopped, _ => State::Idle,
    }
}

fn arm_core(cfg: CaptureConfig) -> Result<pin_owner::SlotId, Error> {
    if state() != State::Idle { return Err(Error::AlreadyCapturing); }
    let channels = cfg.channels.clone();
    let mut pins: heapless::Vec<u32, 32> = heapless::Vec::new();
    if channels.is_empty() { let _ = pins.push(0u32); } else { for &ch in channels.iter() { let _ = pins.push(ch as u32); } }
    let slot = match pin_owner::claim(pin_owner::Driver::Capture, pins.as_slice(), pin_owner::CAP_PIO) {
        Ok(s) => s,
        Err(_) => {
            let mut conflict = cfg.channels.clone();
            if conflict.is_empty() { let mut v = heapless::Vec::<u8, 32>::new(); for &p in pins.iter() { let _ = v.push(p as u8); } conflict = v; }
            return Err(Error::PinConflict { pins: conflict });
        }
    };
    { let mut g = BANK_CLAIM.try_lock().expect("claim"); *g = Some(slot); }
    unsafe { FREQ = cfg.freq; LENGTH = cfg.length; PRE_BYTES = cfg.trigger.pre_size; POST_BYTES = cfg.trigger.post_size; SAMPLE_COUNT = 0; TRIG_PIN = cfg.channels.get(0).copied().unwrap_or(0); TRIGGER_MODE.store(cfg.trigger.r#type as u8, core::sync::atomic::Ordering::Relaxed); }
    STATE.store(State::Armed as u8, Ordering::Relaxed);
    Ok(slot)
}

pub fn arm(cfg: CaptureConfig) -> Result<(), Error> { arm_core(cfg).map(|_| ()) }

pub fn start() -> Result<(), Error> { start_with_trigger(false) }

pub fn start_rising_edge() -> Result<(), Error> { start_with_trigger(true) }

fn start_with_trigger(force_edge: bool) -> Result<(), Error> {
    if state() != State::Armed { return Err(Error::NotArmed); }
    if force_edge { TRIGGER_MODE.store(TriggerType::RisingEdge as u8, core::sync::atomic::Ordering::Relaxed); }
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    Ok(())
}

pub fn stop() {
    let freed = { let mut g = BANK_CLAIM.try_lock().expect("claim"); let s = *g; *g = None; s };
    if let Some(slot) = freed { let _ = pin_owner::release(slot); }
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
    let cfg = CaptureConfig { freq: 1_000_000, length, channels: ch, trigger: TriggerConfig::default() };
    arm(cfg)?;
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    Ok(())
}

pub fn sample_count() -> u32 { unsafe { SAMPLE_COUNT } }

pub fn rising_edge_mode() -> bool { TRIGGER_MODE.load(Ordering::Relaxed) == TriggerType::RisingEdge as u8 }

pub fn bank_word(index: usize) -> u32 {
    if index < unsafe { SAMPLE_COUNT } as usize && index < BANK_WORDS {
        #[allow(static_mut_refs)] unsafe { SAMPLE_RING[index] }
    } else { 0 }
}

pub fn program_names() -> &'static [&'static str] {
    &["BLAST_CAPTURE","POSITIVE_CAPTURE","NEGATIVE_CAPTURE","COMPLEX_CAPTURE","FAST_CAPTURE","NO_TRIGGER_CAPTURE","pio_rle_capture","pio_trigger_qualifier"]
}

pub const fn rle_encode(src: &[u8], dst: &mut [u8], dst_cap: usize) -> usize {
    let mut i: usize = 0; let mut out: usize = 0;
    'outer: while i < src.len() {
        let val = src[i];
        let mut run: usize = 1;
        while i + run < src.len() && run < 255 && src[i + run] == val { run += 1; }
        if out + 2 > dst_cap || out + 2 > dst.len() { break 'outer; }
        dst[out] = run as u8; out += 1;
        dst[out] = val; out += 1;
        i += run;
    }
    out
}

pub const fn rle_decode(src: &[u8], dst: &mut [u8], dst_cap: usize) -> usize {
    let mut inpos: usize = 0; let mut out: usize = 0;
    while inpos + 1 < src.len() {
        let run = src[inpos]; inpos += 1;
        let val = src[inpos]; inpos += 1;
        if out + (run as usize) > dst_cap || out + (run as usize) > dst.len() { break; }
        let mut r: usize = 0;
        while r < (run as usize) { dst[out + r] = val; r += 1; }
        out += run as usize;
    }
    out
}

pub fn bit_delta_transform_u32(src: &[u32], delta_out: &mut [u32], prev_state: &mut u32) {
    let mut prev = *prev_state;
    let limit = src.len().min(delta_out.len());
    let mut i = 0;
    while i < limit { let s = src[i]; delta_out[i] = s ^ prev; prev = s; i += 1; }
    *prev_state = prev;
}

pub fn bit_delta_untransform_u32(delta: &[u32], dest: &mut [u32], prev_state: &mut u32) {
    let mut prev = *prev_state;
    let limit = delta.len().min(dest.len());
    let mut i = 0;
    while i < limit { prev ^= delta[i]; dest[i] = prev; i += 1; }
    *prev_state = prev;
}

pub const TAG_ZERO_SHORT: u8 = 0x00;
pub const TAG_WORD_DELTA: u8 = 0x02;

pub fn bit_delta_compress_block(src_words: &[u32], dst: &mut [u8]) -> usize {
    let mut prev: u32 = 0;
    let mut inpos: usize = 0;
    let mut out: usize = 0;
    let dst_cap = dst.len();
    while inpos < src_words.len() {
        let delta = src_words[inpos] ^ prev;
        if delta == 0 {
            let mut run: usize = 1;
            while inpos + run < src_words.len() && run < 65535 && (src_words[inpos + run] ^ src_words[inpos + run - 1]) == 0 { run += 1; }
            if run <= 255 {
                if out + 2 > dst_cap { return 0; }
                dst[out] = TAG_ZERO_SHORT; out += 1;
                dst[out] = run as u8; out += 1;
            }
            inpos += run;
        } else {
            if out + 5 > dst_cap { return 0; }
            dst[out] = TAG_WORD_DELTA; out += 1;
            dst[out] = (delta & 0xFF) as u8; out += 1;
            dst[out] = ((delta >> 8) & 0xFF) as u8; out += 1;
            dst[out] = ((delta >> 16) & 0xFF) as u8; out += 1;
            dst[out] = ((delta >> 24) & 0xFF) as u8; out += 1;
            prev = src_words[inpos];
            inpos += 1;
        }
    }
    out
}

pub fn bit_delta_decompress_block(src: &[u8], dst_words: &mut [u32]) -> usize {
    let limit = src.len();
    let mut prev: u32 = 0;
    let mut inpos: usize = 0;
    let mut out: usize = 0;
    while inpos < limit {
        let tag = src[inpos]; inpos += 1;
        if tag == TAG_ZERO_SHORT {
            if inpos >= limit { return 0; }
            let run = src[inpos] as usize; inpos += 1;
            if out + run > dst_words.len() { return 0; }
            for r in 0..run { dst_words[out + r] = prev; }
            out += run;
        } else if tag == TAG_WORD_DELTA {
            if inpos + 4 > limit { return 0; }
            let mut delta = src[inpos] as u32; inpos += 1;
            delta |= (src[inpos] as u32) << 8; inpos += 1;
            delta |= (src[inpos] as u32) << 16; inpos += 1;
            delta |= (src[inpos] as u32) << 24; inpos += 1;
            if out >= dst_words.len() { return 0; }
            prev ^= delta; dst_words[out] = prev; out += 1;
        } else {
            return 0;
        }
    }
    out
}

const _: () = {
    let mut dst = [0u8; 32];
    let n = rle_encode(&[1, 1, 2, 3, 3, 3, 3, 3], &mut dst, 32);
    assert!(n == 6);
    let mut dec = [0u8; 32];
    let m = rle_decode(&dst, &mut dec, 32);
    assert!(m == 8);
    assert!(dec[0] == 1 && dec[2] == 2 && dec[4] == 3 && dec[7] == 3);

    // bit-delta self-check is exercised at runtime in hardware builds (callbacks safe) -
    // compile-time `assert!` for these mutation-based helpers is not yet stable.
    let _ = (0u8,);
};

pub fn capture_get_engine_mode_name() -> &'static str { "Raw" }

pub fn generate_vcd(out: &mut heapless::String<2048>) {
    use core::fmt::Write as _;
    let _ = write!(out, "$date Today $end\n$version AgentLA v1.0-rust $end\n$timescale 10ns $end\n$scope module top $end\n");
    let ch_symbols = ['!', '"', '#', '$', '%', '&', '\'', '('];
    for ch in 0..8 {
        let _ = write!(out, "$var wire 1 {} ch{} $end\n", ch_symbols[ch], ch);
    }
    let _ = write!(out, "$upscope $end\n$enddefinitions $end\n#0\n");
    let count = sample_count() as usize;
    let limit = count.min(128);
    let mut prev_word = 0xFFFFFFFFu32;
    for t in 0..limit {
        let w = bank_word(t);
        if w != prev_word {
            let _ = write!(out, "#{}\n", t * 10);
            for ch in 0..8 {
                let bit = (w >> ch) & 1;
                let prev_bit = (prev_word >> ch) & 1;
                if bit != prev_bit || t == 0 {
                    let _ = write!(out, "{}{}\n", bit, ch_symbols[ch]);
                }
            }
            prev_word = w;
        }
    }
}
// ---------- rising-edge trigger qualifier (pure, stateful) ----------
pub fn rising_edge(samples: &[u8], trig_bit: u8) -> usize {
    let mut count = 0usize; let n=samples.len();
    if n==0 { return 0; }
    let mut prev = ((samples[0]>>trig_bit)&1)!=0;
    let mut i=1usize;
    while i<n {
        let cur = ((samples[i]>>trig_bit)&1)!=0;
        if !prev && cur { count += 1; }
        prev = cur;
        i += 1;
    }
    count
}
pub fn finish_captured(slots: u32) { unsafe { SAMPLE_COUNT = slots.max(1); } STATE.store(State::Captured as u8, Ordering::Relaxed); }
pub fn abort_capture() { STATE.store(State::Error as u8, Ordering::Relaxed); }

use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Config, Pio, ShiftConfig, ShiftDirection};
use embassy_rp::dma;
use embassy_rp::peripherals::{DMA_CH0, PIO0};
use embassy_rp::Peri;
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn capture_task(pio0: embassy_rp::Peri<'static, embassy_rp::peripherals::PIO0>, dma0: embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH0>) {
    let Pio { mut common, sm0: mut sm, .. } = Pio::new(pio0, crate::Irqs);
    let mut dma_ref = dma::Channel::new(dma0, crate::Irqs);
    loop {
        if state() == State::Capturing {
            if crate::analyzer::rising_edge_mode() {
                let trpin = unsafe { TRIG_PIN } as usize;
                let mut prev = (pac::SIO.gpio_in(0).read() >> trpin) & 1 != 0;
                let mut guard = 0u32;
                while guard < 4_000_000 {
                    let cur = (pac::SIO.gpio_in(0).read() >> trpin) & 1 != 0;
                    if !prev && cur { break; }
                    prev = cur;
                    guard += 1;
                    if guard.rem_euclid(1000) == 0 { Timer::after(Duration::from_micros(1)).await; }
                }
            }
            let mut bank = [0u32; BANK_WORDS];
            let prg = pio_asm!(
                ".wrap_target",
                " in pins 32",
                ".wrap",
            );
            let mut cfg = Config::default();
            cfg.use_program(&common.load_program(&prg.program), &[]);

            // Dynamic clock divider based on requested sampling frequency (sys_clk = 150 MHz)
            let freq = unsafe { FREQ }.max(10_000).min(150_000_000);
            let div_int = (150_000_000 / freq).max(1).min(65535) as u16;
            let div_frac = (((150_000_000 % freq) * 256) / freq).min(255) as u8;

            cfg.clock_divider = 1u8.into();
            cfg.shift_in = ShiftConfig { auto_fill: true, threshold: 32, direction: ShiftDirection::Left };
            sm.set_config(&cfg);
            // Apply exact integer + fractional clock division
            pac::PIO0.sm(0).clkdiv().write(|w| {
                w.set_int(div_int);
                w.set_frac(div_frac);
            });
            sm.set_enable(true);
            {
                let (rx, _tx) = sm.rx_tx();
                rx.dma_pull(&mut dma_ref, &mut bank, false).await;
                for i in 0..bank.len() {
                    unsafe { SAMPLE_RING[i] = bank[i]; }
                }
                finish_captured(bank.len() as u32);
            }
            sm.set_enable(false);
            while state() != State::Idle {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
            }
        } else {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(5)).await;
        }
    }
}
