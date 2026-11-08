//! Inter-Core Lock-Free SPSC Ring Buffer & Streaming Frame Architecture.
//! Enables Core 1 hard-real-time capture & DSP compression to feed Core 0 network tasks.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicU16, AtomicU32, AtomicUsize, Ordering};

pub const FRAME_MAGIC: [u8; 2] = [0xA5, 0x5A];

pub const FLAG_OVERFLOW: u8 = 0x01;
pub const FLAG_FORMAT_SUMP_RLE: u8 = 0x02;
pub const FLAG_FORMAT_BIT_DELTA: u8 = 0x00;

/// Header prepended to every stream packet (8 bytes, naturally aligned)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamFrameHeader {
    pub magic: [u8; 2],
    pub seq: u16,
    pub flags: u8,
    pub channels: u8,
    pub payload_len: u16,
}

impl StreamFrameHeader {
    pub const SIZE: usize = 8;

    pub fn new(seq: u16, flags: u8, channels: u8, payload_len: u16) -> Self {
        Self {
            magic: FRAME_MAGIC,
            seq,
            flags,
            channels,
            payload_len,
        }
    }

    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut b = [0u8; Self::SIZE];
        b[0] = self.magic[0];
        b[1] = self.magic[1];
        let seq_b = self.seq.to_le_bytes();
        b[2] = seq_b[0];
        b[3] = seq_b[1];
        b[4] = self.flags;
        b[5] = self.channels;
        let len_b = self.payload_len.to_le_bytes();
        b[6] = len_b[0];
        b[7] = len_b[1];
        b
    }

    pub fn from_bytes(b: &[u8; Self::SIZE]) -> Option<Self> {
        if b[0] != FRAME_MAGIC[0] || b[1] != FRAME_MAGIC[1] {
            return None;
        }
        Some(Self {
            magic: [b[0], b[1]],
            seq: u16::from_le_bytes([b[2], b[3]]),
            flags: b[4],
            channels: b[5],
            payload_len: u16::from_le_bytes([b[6], b[7]]),
        })
    }
}

/// A 32 KiB Lock-free Single-Producer Single-Consumer (SPSC) Circular Queue.
/// Core 1 pushes compressed frames; Core 0 drains frames to network clients.
pub struct SpscStreamBuffer<const CAPACITY: usize> {
    buffer: UnsafeCell<[u8; CAPACITY]>,
    head: AtomicUsize,
    tail: AtomicUsize,
    seq: AtomicU16,
    overflow_count: AtomicU32,
    had_overflow: AtomicU32,
}

unsafe impl<const CAPACITY: usize> Sync for SpscStreamBuffer<CAPACITY> {}

impl<const CAPACITY: usize> SpscStreamBuffer<CAPACITY> {
    pub const fn new() -> Self {
        Self {
            buffer: UnsafeCell::new([0u8; CAPACITY]),
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            seq: AtomicU16::new(0),
            overflow_count: AtomicU32::new(0),
            had_overflow: AtomicU32::new(0),
        }
    }

    pub fn reset(&self) {
        self.head.store(0, Ordering::Release);
        self.tail.store(0, Ordering::Release);
        self.seq.store(0, Ordering::Release);
        self.overflow_count.store(0, Ordering::Release);
        self.had_overflow.store(0, Ordering::Release);
    }

    pub fn available_to_read(&self) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        if head >= tail {
            head - tail
        } else {
            CAPACITY - (tail - head)
        }
    }

    pub fn available_to_write(&self) -> usize {
        CAPACITY - 1 - self.available_to_read()
    }

    /// Producer (Core 1): Push a frame with automatic header wrapping.
    /// Returns true if pushed, or false if buffer full (records overflow).
    pub fn push_frame(&self, flags: u8, channels: u8, payload: &[u8]) -> bool {
        let total_needed = StreamFrameHeader::SIZE + payload.len();
        let avail = self.available_to_write();

        if total_needed > avail {
            // Buffer overflow: increment drop counters
            self.overflow_count.fetch_add(1, Ordering::Relaxed);
            self.had_overflow.store(1, Ordering::Relaxed);
            return false;
        }

        let seq = self.seq.fetch_add(1, Ordering::Relaxed);
        let mut final_flags = flags;
        if self.had_overflow.swap(0, Ordering::Relaxed) != 0 {
            final_flags |= FLAG_OVERFLOW;
        }

        let header = StreamFrameHeader::new(seq, final_flags, channels, payload.len() as u16);
        let hdr_bytes = header.to_bytes();

        let mut head = self.head.load(Ordering::Relaxed);
        let buf_ptr = self.buffer.get();

        unsafe {
            // Copy header
            for &b in &hdr_bytes {
                (*buf_ptr)[head] = b;
                head = (head + 1) % CAPACITY;
            }

            // Copy payload
            for &b in payload {
                (*buf_ptr)[head] = b;
                head = (head + 1) % CAPACITY;
            }
        }

        self.head.store(head, Ordering::Release);
        true
    }

    /// Pushes raw bytes directly into the queue without a frame header.
    pub fn push_raw(&self, data: &[u8]) -> bool {
        let avail = self.available_to_write();
        if data.len() > avail {
            return false;
        }
        let buf_ptr = self.buffer.get();
        let mut head = self.head.load(Ordering::Relaxed);
        unsafe {
            for &b in data {
                (*buf_ptr)[head] = b;
                head = (head + 1) % CAPACITY;
            }
        }
        self.head.store(head, Ordering::Release);
        true
    }

    /// Consumer (Core 0): Pop up to `dst.len()` raw bytes from the stream buffer.
    /// Returns actual number of bytes read.
    pub fn pop_bytes(&self, dst: &mut [u8]) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let mut tail = self.tail.load(Ordering::Relaxed);

        let available = if head >= tail {
            head - tail
        } else {
            CAPACITY - (tail - head)
        };

        let to_read = available.min(dst.len());
        let buf_ptr = self.buffer.get();

        unsafe {
            for i in 0..to_read {
                dst[i] = (*buf_ptr)[tail];
                tail = (tail + 1) % CAPACITY;
            }
        }

        self.tail.store(tail, Ordering::Release);
        to_read
    }

    /// Consumer (OLS Server): Pop a single complete frame's raw payload, stripping
    /// the internal `StreamFrameHeader`.
    /// Returns number of payload bytes written to `dst`.
    pub fn pop_payload(&self, dst: &mut [u8]) -> usize {
        let head = self.head.load(Ordering::Acquire);
        let mut tail = self.tail.load(Ordering::Relaxed);

        let available = if head >= tail {
            head - tail
        } else {
            CAPACITY - (tail - head)
        };

        if available < StreamFrameHeader::SIZE {
            return 0;
        }

        let buf_ptr = self.buffer.get();
        let mut hdr = [0u8; StreamFrameHeader::SIZE];
        unsafe {
            for i in 0..StreamFrameHeader::SIZE {
                hdr[i] = (*buf_ptr)[(tail + i) % CAPACITY];
            }
        }

        if hdr[0] != FRAME_MAGIC[0] || hdr[1] != FRAME_MAGIC[1] {
            // Out of sync: advance by 1 to re-align
            self.tail.store((tail + 1) % CAPACITY, Ordering::Release);
            return 0;
        }

        let payload_len = u16::from_le_bytes([hdr[6], hdr[7]]) as usize;
        let total_frame_len = StreamFrameHeader::SIZE + payload_len;

        if available < total_frame_len {
            // Entire frame not yet fully written
            return 0;
        }

        // Advance tail past header
        tail = (tail + StreamFrameHeader::SIZE) % CAPACITY;

        let to_copy = payload_len.min(dst.len());
        unsafe {
            for i in 0..to_copy {
                dst[i] = (*buf_ptr)[(tail + i) % CAPACITY];
            }
        }

        // Advance tail past entire payload
        self.tail.store((tail + payload_len) % CAPACITY, Ordering::Release);
        to_copy
    }

    pub fn overflow_count(&self) -> u32 {
        self.overflow_count.load(Ordering::Relaxed)
    }
}
