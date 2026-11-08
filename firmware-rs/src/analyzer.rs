//! Logic analyzer core + hardware capture task for RP2350.
//! Algorithmic compression, VCD generation, and state types are delegated to `agentla-analyzer`.

use core::sync::atomic::{AtomicU8, Ordering};

#[allow(unused_imports)]
pub use agentla_analyzer::trigger::{falling_edge, rising_edge};
#[allow(unused_imports)]
pub use agentla_analyzer::{
    bit_delta_compress_block, bit_delta_decompress_block, bit_delta_transform_u32,
    bit_delta_untransform_u32, capture_get_engine_mode_name, program_names, rle_decode,
    rle_encode, CaptureConfig, Error, Mode, State, Status, TriggerConfig, TriggerType,
    BANK_WORDS, SAMPLE_COUNT_MAX, TAG_WORD_DELTA, TAG_ZERO_SHORT,
};

use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::pin_owner;
use rp_pac as pac;

static STATE: AtomicU8 = AtomicU8::new(State::Idle as u8);
static BANK_CLAIM: Mutex<CriticalSectionRawMutex, Option<pin_owner::SlotId>> = Mutex::new(None);

pub static STREAM_ACTIVE: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);
pub static STREAM_CHANNELS: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(8);
pub static STREAM_FORMAT: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(agentla_analyzer::FLAG_FORMAT_BIT_DELTA);
pub static STREAM_RING: agentla_analyzer::SpscStreamBuffer<32768> = agentla_analyzer::SpscStreamBuffer::new();
pub static STREAM_SIGNAL: embassy_sync::signal::Signal<CriticalSectionRawMutex, ()> = embassy_sync::signal::Signal::new();

pub static TRIGGER_MATRIX: Mutex<CriticalSectionRawMutex, agentla_analyzer::decoders::matrix::HardwareTriggerMatrix> =
    Mutex::new(agentla_analyzer::decoders::matrix::HardwareTriggerMatrix::new());
pub static DECODED_RING: agentla_analyzer::SpscStreamBuffer<4096> = agentla_analyzer::SpscStreamBuffer::new();
pub static DECODED_SIGNAL: embassy_sync::signal::Signal<CriticalSectionRawMutex, ()> = embassy_sync::signal::Signal::new();

#[derive(Clone, Copy, Debug, Default)]
pub struct DecoderRuntimeConfig {
    pub enabled: bool,
    pub kind: u8,
    pub pin_a: u8,
    pub pin_b: u8,
    pub baud: u32,
}

pub static ACTIVE_DECODER: Mutex<CriticalSectionRawMutex, DecoderRuntimeConfig> =
    Mutex::new(DecoderRuntimeConfig {
        enabled: false,
        kind: 0,
        pin_a: 0,
        pin_b: 0,
        baud: 115_200,
    });

pub fn emit_decoded_event(event: &agentla_analyzer::decoders::event::DecodedEvent) {
    let mut json_buf = [0u8; 256];
    let len = event.format_json(&mut json_buf);
    if len > 0 {
        let _ = DECODED_RING.push_raw(&json_buf[..len]);
        // Evaluate hardware trigger matrix
        if let Ok(matrix) = TRIGGER_MATRIX.try_lock() {
            if let Some(act) = matrix.evaluate(event) {
                if act.freeze_capture {
                    STATE.store(State::Captured as u8, Ordering::Relaxed);
                }
            }
        }
        DECODED_SIGNAL.signal(());
    }
}

static mut PING_PONG_A: [u32; 1024] = [0; 1024];
static mut PING_PONG_B: [u32; 1024] = [0; 1024];
static STREAM_CLAIM: Mutex<CriticalSectionRawMutex, Option<pin_owner::SlotId>> = Mutex::new(None);

static mut FREQ: u32 = 1_000_000;
static mut LENGTH: u32 = 0;
static mut PRE_BYTES: u32 = 0;
static mut POST_BYTES: u32 = 0;
static mut SAMPLE_COUNT: u32 = 0;
static mut TRIG_PIN: u8 = 0;
static TRIGGER_MODE: core::sync::atomic::AtomicU8 = core::sync::atomic::AtomicU8::new(0);
static mut SAMPLE_RING: [u32; BANK_WORDS] = [0; BANK_WORDS];

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

fn arm_core(cfg: CaptureConfig) -> Result<pin_owner::SlotId, Error> {
    if state() != State::Idle {
        return Err(Error::AlreadyCapturing);
    }
    let channels = cfg.channels.clone();
    let mut pins: heapless::Vec<u32, 32> = heapless::Vec::new();
    if channels.is_empty() {
        let _ = pins.push(0u32);
    } else {
        for &ch in channels.iter() {
            let _ = pins.push(ch as u32);
        }
    }
    let slot = match pin_owner::claim(pin_owner::Driver::Capture, pins.as_slice(), pin_owner::CAP_PIO) {
        Ok(s) => s,
        Err(_) => {
            let mut conflict = cfg.channels.clone();
            if conflict.is_empty() {
                let mut v = heapless::Vec::<u8, 32>::new();
                for &p in pins.iter() {
                    let _ = v.push(p as u8);
                }
                conflict = v;
            }
            return Err(Error::PinConflict { pins: conflict });
        }
    };
    {
        let mut g = BANK_CLAIM.try_lock().expect("claim");
        *g = Some(slot);
    }
    unsafe {
        FREQ = cfg.freq;
        LENGTH = cfg.length;
        PRE_BYTES = cfg.trigger.pre_size;
        POST_BYTES = cfg.trigger.post_size;
        SAMPLE_COUNT = 0;
        TRIG_PIN = cfg.channels.first().copied().unwrap_or(0);
        TRIGGER_MODE.store(cfg.trigger.r#type as u8, core::sync::atomic::Ordering::Relaxed);
    }
    STATE.store(State::Armed as u8, Ordering::Relaxed);
    Ok(slot)
}

pub fn arm(cfg: CaptureConfig) -> Result<(), Error> {
    arm_core(cfg).map(|_| ())
}

pub fn start() -> Result<(), Error> {
    start_with_trigger(false)
}

pub fn start_rising_edge() -> Result<(), Error> {
    start_with_trigger(true)
}

fn start_with_trigger(force_edge: bool) -> Result<(), Error> {
    if state() != State::Armed {
        return Err(Error::NotArmed);
    }
    if force_edge {
        TRIGGER_MODE.store(TriggerType::RisingEdge as u8, core::sync::atomic::Ordering::Relaxed);
    }
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    Ok(())
}

pub fn stop() {
    let freed = {
        let mut g = BANK_CLAIM.try_lock().expect("claim");
        let s = *g;
        *g = None;
        s
    };
    if let Some(slot) = freed {
        let _ = pin_owner::release(slot);
    }
    STATE.store(State::Idle as u8, Ordering::Relaxed);
}

pub fn core1_capture_loop() -> ! {
    let mut comp_buf = [0u8; 2048];
    let mut use_buf_a = true;

    loop {
        if !STREAM_ACTIVE.load(Ordering::Relaxed) {
            cortex_m::asm::delay(50_000);
            continue;
        }

        let channels = STREAM_CHANNELS.load(Ordering::Relaxed).max(4).min(16);
        let format = STREAM_FORMAT.load(Ordering::Relaxed);

        let slice: &mut [u32] = unsafe {
            let ptr = if use_buf_a {
                core::ptr::addr_of_mut!(PING_PONG_A) as *mut [u32; 1024]
            } else {
                core::ptr::addr_of_mut!(PING_PONG_B) as *mut [u32; 1024]
            };
            &mut *ptr
        };

        // Real-time hardware GPIO sampling across active channel width
        let base_shift = if channels == 16 { 10 } else { 16 };
        let mask = (1u32 << channels) - 1;
        for i in 0..1024 {
            let gpio = pac::SIO.gpio_in(0).read();
            slice[i] = (gpio >> base_shift) & mask;
            cortex_m::asm::delay(20);
        }

        let enc_len = if format == agentla_analyzer::FLAG_FORMAT_SUMP_RLE {
            let byte_slice = unsafe {
                core::slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * 4)
            };
            agentla_analyzer::sump_rle_encode_8bit(byte_slice, &mut comp_buf)
        } else {
            agentla_analyzer::bit_delta_compress_block(slice, &mut comp_buf)
        };

        if enc_len > 0 {
            STREAM_RING.push_frame(format, channels, &comp_buf[..enc_len]);
            STREAM_SIGNAL.signal(());
        }

        use_buf_a = !use_buf_a;
        cortex_m::asm::delay(500);
    }
}

pub fn start_continuous_stream(channels: u8, format: u8, freq_hz: u32) -> Result<(), Error> {
    if STREAM_ACTIVE.load(Ordering::Relaxed) {
        return Ok(());
    }

    let ch_count = match channels {
        4 => 4,
        16 => 16,
        _ => 8,
    };

    let base_pin: u32 = if ch_count == 16 { 10 } else { 16 };
    let mut pins = heapless::Vec::<u32, 16>::new();
    for i in 0..ch_count {
        let pin = if ch_count == 16 && (base_pin + i as u32) >= 25 {
            // Pin 25 is LED; skip to 26
            base_pin + i as u32 + 1
        } else {
            base_pin + i as u32
        };
        let _ = pins.push(pin);
    }

    let slot = pin_owner::claim(pin_owner::Driver::Capture, pins.as_slice(), pin_owner::CAP_PIO)
        .map_err(|_| Error::PinConflict { pins: heapless::Vec::new() })?;

    {
        let mut g = STREAM_CLAIM.try_lock().expect("claim");
        *g = Some(slot);
    }

    STREAM_RING.reset();
    STREAM_CHANNELS.store(ch_count as u8, Ordering::Relaxed);
    STREAM_FORMAT.store(format, Ordering::Relaxed);
    unsafe { FREQ = freq_hz.max(10_000).min(100_000_000); }
    STREAM_ACTIVE.store(true, Ordering::Release);
    Ok(())
}

pub fn stop_continuous_stream() {
    STREAM_ACTIVE.store(false, Ordering::Release);
    let freed = {
        let mut g = STREAM_CLAIM.try_lock().expect("claim");
        let s = *g;
        *g = None;
        s
    };
    if let Some(slot) = freed {
        let _ = pin_owner::release(slot);
    }
}

pub fn read_stream_chunk(dst: &mut [u8]) -> usize {
    STREAM_RING.pop_bytes(dst)
}

pub fn read_stream_payload(dst: &mut [u8]) -> usize {
    STREAM_RING.pop_payload(dst)
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
    for &c in channels {
        let _ = ch.push(c);
    }
    let cfg = CaptureConfig {
        freq: 1_000_000,
        length,
        channels: ch,
        trigger: TriggerConfig::default(),
    };
    arm(cfg)?;
    STATE.store(State::Capturing as u8, Ordering::Relaxed);
    Ok(())
}

pub fn sample_count() -> u32 {
    unsafe { SAMPLE_COUNT }
}

pub fn rising_edge_mode() -> bool {
    TRIGGER_MODE.load(Ordering::Relaxed) == TriggerType::RisingEdge as u8
}

pub fn bank_word(index: usize) -> u32 {
    if index < unsafe { SAMPLE_COUNT } as usize && index < BANK_WORDS {
        #[allow(static_mut_refs)]
        unsafe {
            SAMPLE_RING[index]
        }
    } else {
        0
    }
}

pub fn generate_vcd(out: &mut heapless::String<2048>) {
    let count = sample_count() as usize;
    let limit = count.min(128);
    #[allow(static_mut_refs)]
    let slice = unsafe { &SAMPLE_RING[..limit] };
    agentla_analyzer::generate_vcd(slice, out);
}

pub fn finish_captured(slots: u32) {
    unsafe {
        SAMPLE_COUNT = slots.max(1);
    }
    STATE.store(State::Captured as u8, Ordering::Relaxed);
}

pub fn abort_capture() {
    STATE.store(State::Error as u8, Ordering::Relaxed);
}

use embassy_rp::dma;
use embassy_rp::pio::program::pio_asm;
use embassy_rp::pio::{Config, Pio, ShiftConfig, ShiftDirection};
use embassy_time::{Duration, Timer};

#[embassy_executor::task]
pub async fn capture_task(
    pio0: embassy_rp::Peri<'static, embassy_rp::peripherals::PIO0>,
    dma0: embassy_rp::Peri<'static, embassy_rp::peripherals::DMA_CH0>,
) {
    let Pio {
        mut common,
        sm0: mut sm,
        ..
    } = Pio::new(pio0, crate::Irqs);
    let mut dma_ref = dma::Channel::new(dma0, crate::Irqs);
    loop {
        if state() == State::Capturing {
            if crate::analyzer::rising_edge_mode() {
                let trpin = unsafe { TRIG_PIN } as usize;
                let mut prev = (pac::SIO.gpio_in(0).read() >> trpin) & 1 != 0;
                let mut guard = 0u32;
                while guard < 4_000_000 {
                    let cur = (pac::SIO.gpio_in(0).read() >> trpin) & 1 != 0;
                    if !prev && cur {
                        break;
                    }
                    prev = cur;
                    guard += 1;
                    if guard.rem_euclid(1000) == 0 {
                        Timer::after(Duration::from_micros(1)).await;
                    }
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

            let sys_clk = crate::benchmark::get_sys_clk_hz();
            let freq = unsafe { FREQ }.max(10_000).min(sys_clk);
            let div_int = (sys_clk / freq).max(1).min(65535) as u16;
            let div_frac = (((sys_clk % freq) * 256) / freq).min(255) as u8;

            cfg.clock_divider = 1u8.into();
            cfg.shift_in = ShiftConfig {
                auto_fill: true,
                threshold: 32,
                direction: ShiftDirection::Left,
            };
            sm.set_config(&cfg);

            pac::PIO0.sm(0).clkdiv().write(|w| {
                w.set_int(div_int);
                w.set_frac(div_frac);
            });
            sm.set_enable(true);
            {
                let (rx, _tx) = sm.rx_tx();
                rx.dma_pull(&mut dma_ref, &mut bank, false).await;
                for i in 0..bank.len() {
                    unsafe {
                        SAMPLE_RING[i] = bank[i];
                    }
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

// ============================================================================
// Autonomous Bus Sniffer Engine
// ============================================================================

pub fn sniff_bus(protocol: &str, duration_ms: u32) -> Result<heapless::String<1536>, &'static str> {
    use core::fmt::Write as _;
    let dur = duration_ms.clamp(50, 1000);
    let is_i2c = protocol.eq_ignore_ascii_case("i2c");
    let is_spi = protocol.eq_ignore_ascii_case("spi");

    if !is_i2c && !is_spi {
        return Err("unsupported protocol (supported: i2c, spi)");
    }

    if is_i2c {
        let pins = [16u32, 17u32];
        let slot = pin_owner::claim(pin_owner::Driver::Capture, &pins, pin_owner::CAP_SIO)
            .map_err(|_| "pins [16, 17] already claimed by an active peripheral driver or programmer session")?;

        let mut samples = [0u8; 1024];
        let count = samples.len();
        for i in 0..count {
            let gpio_in = pac::SIO.gpio_in(0).read();
            samples[i] = ((gpio_in >> 16) & 3) as u8;
            cortex_m::asm::delay(150);
        }

        let _ = pin_owner::release(slot);

        let mut txs = heapless::Vec::<crate::decoder::I2cDecodedTransaction, 32>::new();
        let total_tx = crate::decoder::i2c_decode_transactions(&samples, 1, 0, &mut txs);

        let mut addrs = heapless::Vec::<u8, 16>::new();
        for tx in txs.iter() {
            if !addrs.contains(&tx.addr) {
                let _ = addrs.push(tx.addr);
            }
        }

        let mut out = heapless::String::<1536>::new();
        let _ = write!(
            out,
            r#"{{"protocol":"i2c","duration_ms":{},"total_transactions":{},"active_addresses":["#,
            dur, total_tx
        );
        for (i, &a) in addrs.iter().enumerate() {
            if i > 0 {
                let _ = out.push(',');
            }
            let _ = write!(out, r#""0x{:02x}""#, a);
        }
        let _ = write!(out, r#"],"transactions":["#);
        for (i, tx) in txs.iter().enumerate() {
            if i > 0 {
                let _ = out.push(',');
            }
            let dir = if tx.is_read { "read" } else { "write" };
            let _ = write!(
                out,
                r#"{{"address":"0x{:02x}","direction":"{}","ack":{},"data":""#,
                tx.addr, dir, tx.ack
            );
            for d in 0..tx.data_len as usize {
                let _ = write!(out, "{:02x}", tx.data[d]);
            }
            let _ = write!(out, r#""}}"#);
        }
        let _ = write!(out, r#"]}}"#);
        Ok(out)
    } else {
        let pins = [18u32, 19u32, 20u32, 21u32];
        let slot = pin_owner::claim(pin_owner::Driver::Capture, &pins, pin_owner::CAP_SIO)
            .map_err(|_| "pins [18, 19, 20, 21] already claimed by an active peripheral driver or programmer session")?;

        let mut samples = [0u8; 1024];
        let count = samples.len();
        for i in 0..count {
            let gpio_in = pac::SIO.gpio_in(0).read();
            samples[i] = ((gpio_in >> 18) & 0x0F) as u8;
            cortex_m::asm::delay(150);
        }

        let _ = pin_owner::release(slot);

        let mut txs = heapless::Vec::<crate::decoder::SpiDecodedTransaction, 32>::new();
        let total_tx = crate::decoder::spi_decode_transactions(&samples, 0, 1, 2, 3, &mut txs);

        let mut out = heapless::String::<1536>::new();
        let _ = write!(
            out,
            r#"{{"protocol":"spi","duration_ms":{},"total_transactions":{},"transactions":["#,
            dur, total_tx
        );
        for (i, tx) in txs.iter().enumerate() {
            if i > 0 {
                let _ = out.push(',');
            }
            let _ = write!(out, r#"{{"mosi":""#);
            for d in 0..tx.data_len as usize {
                let _ = write!(out, "{:02x}", tx.mosi[d]);
            }
            let _ = write!(out, r#"","miso":""#);
            for d in 0..tx.data_len as usize {
                let _ = write!(out, "{:02x}", tx.miso[d]);
            }
            let _ = write!(out, r#""}}"#);
        }
        let _ = write!(out, r#"]}}"#);
        Ok(out)
    }
}
