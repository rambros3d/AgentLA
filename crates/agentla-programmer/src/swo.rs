//! Serial Wire Output (SWO) and Instrumentation Trace Macrocell (ITM) trace engine.
//!
//! Provides a lock-free circular ring buffer capturing asynchronous NRZ trace
//! packets from ARM CoreSight ITM / DWT hardware, with strict packet pacing
//! for ST-Link / USB endpoints and half-buffer overrun drop protection.

use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU8, Ordering};

/// Capacity of the SWO circular ring buffer in bytes.
pub const SWO_BUF_SIZE: usize = 4096;

/// Default baud rate for SWO capture (2.0 Mbaud).
pub const SWO_DEFAULT_BAUD: u32 = 2_000_000;

/// Maximum supported SWO baud rate (4.5 Mbaud).
pub const SWO_MAX_BAUD: u32 = 4_500_000;

/// Overrun threshold: if unconsumed bytes exceed this, oldest half is dropped.
pub const SWO_OVERRUN_THRESHOLD: u32 = (SWO_BUF_SIZE - 256) as u32; // 3840

/// Overrun target retention: keep newest half of the buffer on overrun.
pub const SWO_DROP_TARGET: u32 = (SWO_BUF_SIZE / 2) as u32; // 2048

/// Live telemetry reported by the SWO receiver engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwoTelemetry {
    pub active: bool,
    pub baud_rate: u32,
    pub bytes_received: u32,
    pub bytes_delivered: u32,
    pub bytes_dropped: u32,
}

const ATOMIC_ZERO: AtomicU8 = AtomicU8::new(0);

/// Lock-free circular SWO capture engine and ring buffer.
pub struct SwoEngine {
    buf: [AtomicU8; SWO_BUF_SIZE],
    head: AtomicU32,
    tail: AtomicU32,
    dropped: AtomicU32,
    delivered: AtomicU32,
    baud_rate: AtomicU32,
    active: AtomicBool,
    trace_owed: AtomicU32,
    trace_limit: AtomicU32,
    trace_polled: AtomicBool,
}

impl Default for SwoEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SwoEngine {
    /// Create a new uninitialized SWO engine with 4 KiB ring buffer.
    pub const fn new() -> Self {
        Self {
            buf: [ATOMIC_ZERO; SWO_BUF_SIZE],
            head: AtomicU32::new(0),
            tail: AtomicU32::new(0),
            dropped: AtomicU32::new(0),
            delivered: AtomicU32::new(0),
            baud_rate: AtomicU32::new(SWO_DEFAULT_BAUD),
            active: AtomicBool::new(false),
            trace_owed: AtomicU32::new(0),
            trace_limit: AtomicU32::new(0),
            trace_polled: AtomicBool::new(false),
        }
    }

    /// Reset all counters, buffers, and tracking state.
    pub fn reset(&self) {
        self.active.store(false, Ordering::Release);
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        self.dropped.store(0, Ordering::Release);
        self.delivered.store(0, Ordering::Release);
        self.trace_owed.store(0, Ordering::Release);
        self.trace_limit.store(0, Ordering::Release);
        self.trace_polled.store(false, Ordering::Release);
        self.baud_rate.store(SWO_DEFAULT_BAUD, Ordering::Release);
    }

    /// Start SWO capture with configured baud rate and optional host buffer limit.
    pub fn start(&self, baud: u32, host_limit: u16) -> bool {
        if baud == 0 || baud > SWO_MAX_BAUD {
            return false;
        }
        self.baud_rate.store(baud, Ordering::Release);
        self.trace_limit.store(host_limit as u32, Ordering::Release);
        self.trace_owed.store(0, Ordering::Release);
        self.trace_polled.store(false, Ordering::Release);
        self.active.store(true, Ordering::Release);
        true
    }

    /// Stop SWO capture.
    pub fn stop(&self) {
        self.active.store(false, Ordering::Release);
        self.trace_owed.store(0, Ordering::Release);
        self.trace_polled.store(false, Ordering::Release);
    }

    /// Check if SWO capture is currently active.
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Check configured baud rate.
    pub fn baud_rate(&self) -> u32 {
        self.baud_rate.load(Ordering::Acquire)
    }

    /// Push a single received trace byte into the circular ring buffer.
    pub fn push_byte(&self, byte: u8) {
        if !self.is_active() {
            return;
        }
        let head = self.head.load(Ordering::Acquire);
        let idx = (head as usize) % SWO_BUF_SIZE;
        self.buf[idx].store(byte, Ordering::Release);
        let new_head = head.wrapping_add(1);
        self.head.store(new_head, Ordering::Release);

        self.check_overrun(new_head);
    }

    /// Push a slice of received trace bytes into the circular ring buffer.
    pub fn push_bytes(&self, bytes: &[u8]) {
        if !self.is_active() {
            return;
        }
        for &b in bytes {
            self.push_byte(b);
        }
    }

    /// Overrun drop logic: if unconsumed bytes exceed SWO_OVERRUN_THRESHOLD (3840 bytes),
    /// drop oldest bytes to retain exactly SWO_DROP_TARGET (2048 bytes, half the buffer).
    fn check_overrun(&self, current_head: u32) {
        let mut tail = self.tail.load(Ordering::Acquire);
        loop {
            let n = current_head.wrapping_sub(tail);
            if n <= SWO_OVERRUN_THRESHOLD {
                break;
            }
            let drop = n - SWO_DROP_TARGET;
            let new_tail = tail.wrapping_add(drop);
            match self.tail.compare_exchange_weak(tail, new_tail, Ordering::AcqRel, Ordering::Acquire) {
                Ok(_) => {
                    self.dropped.fetch_add(drop, Ordering::Relaxed);
                    break;
                }
                Err(actual_tail) => {
                    tail = actual_tail;
                }
            }
        }
    }

    /// Number of unconsumed trace bytes currently available in the ring buffer.
    pub fn available(&self) -> usize {
        if !self.is_active() {
            return 0;
        }
        let head = self.head.load(Ordering::Acquire);
        self.check_overrun(head);
        let tail = self.tail.load(Ordering::Acquire);
        let n = head.wrapping_sub(tail);
        n as usize
    }

    /// Peek up to dst.len() bytes without advancing the tail pointer.
    pub fn peek(&self, dst: &mut [u8]) -> usize {
        let avail = self.available();
        if avail == 0 || dst.is_empty() {
            return 0;
        }
        let to_copy = avail.min(dst.len());
        let tail = self.tail.load(Ordering::Acquire);
        for i in 0..to_copy {
            let idx = (tail.wrapping_add(i as u32) as usize) % SWO_BUF_SIZE;
            dst[i] = self.buf[idx].load(Ordering::Acquire);
        }
        to_copy
    }

    /// Advance the tail pointer by n bytes, recording delivered bytes.
    pub fn consume(&self, n: usize) -> usize {
        let avail = self.available();
        let to_consume = (n as u32).min(avail as u32);
        if to_consume > 0 {
            self.tail.fetch_add(to_consume, Ordering::AcqRel);
            self.delivered.fetch_add(to_consume, Ordering::Relaxed);
        }
        to_consume as usize
    }

    /// Read and consume up to dst.len() bytes.
    pub fn read(&self, dst: &mut [u8]) -> usize {
        let n = self.peek(dst);
        if n > 0 {
            self.consume(n);
        }
        n
    }

    /// Reset trace pacing and host buffer limit (matching hybridlink stlink_trace_reset).
    pub fn stlink_trace_reset(&self, host_buf: u32) {
        self.trace_owed.store(0, Ordering::Release);
        self.trace_polled.store(false, Ordering::Release);
        self.trace_limit.store(host_buf, Ordering::Release);
    }

    /// Calculate and promise trace count for ST-Link DBG_APIV2_GET_TRACE_NB.
    pub fn stlink_trace_count(&self) -> u32 {
        if !self.is_active() {
            return 0;
        }
        let polled = self.trace_polled.load(Ordering::Acquire);
        let owed = self.trace_owed.load(Ordering::Acquire);
        if polled && owed > 0 {
            return owed;
        }

        self.trace_polled.store(true, Ordering::Release);
        let mut n = self.available() as u32;
        let limit = self.trace_limit.load(Ordering::Acquire);
        if limit > 0 && n > limit {
            n = limit;
        }
        if n > 0xFFFF {
            n = 0xFFFF;
        }
        self.trace_owed.store(n, Ordering::Release);
        n
    }

    /// Returns true if polled mode is active.
    pub fn is_polled(&self) -> bool {
        self.trace_polled.load(Ordering::Acquire)
    }

    /// Returns the number of bytes currently owed to the host.
    pub fn trace_owed(&self) -> u32 {
        self.trace_owed.load(Ordering::Acquire)
    }

    /// Pop next packet for USB endpoint transmission.
    ///
    /// In polled mode (`GET_TRACE_NB`), guarantees delivering exactly `trace_owed` bytes
    /// in chunks matching `dst.len()` (typically 64 bytes). If an overrun occurred
    /// after the count was promised, remaining bytes in the promised packet are padded
    /// with zero (CoreSight ITM sync frames) to protect host tools against fatal USB short-packet mismatch.
    pub fn pop_trace_packet(&self, dst: &mut [u8]) -> usize {
        if !self.is_active() || dst.is_empty() {
            return 0;
        }
        if self.trace_polled.load(Ordering::Acquire) {
            let owed = self.trace_owed.load(Ordering::Acquire);
            if owed == 0 {
                return 0;
            }
            let want = (owed as usize).min(dst.len());
            let n = self.read(&mut dst[..want]);
            if n < want {
                // Pad with ITM sync bytes to keep promised byte count strictly exact
                dst[n..want].fill(0);
                self.delivered.fetch_add((want - n) as u32, Ordering::Relaxed);
            }
            self.trace_owed.fetch_sub(want as u32, Ordering::AcqRel);
            want
        } else {
            // Streaming mode: pop up to dst.len() available bytes
            self.read(dst)
        }
    }

    /// Get current live telemetry snapshot.
    pub fn telemetry(&self) -> SwoTelemetry {
        SwoTelemetry {
            active: self.is_active(),
            baud_rate: self.baud_rate(),
            bytes_received: self.head.load(Ordering::Acquire),
            bytes_delivered: self.delivered.load(Ordering::Acquire),
            bytes_dropped: self.dropped.load(Ordering::Acquire),
        }
    }
}
