use core::fmt::Write as _;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use core::cell::RefCell;

use crate::programmer::target::cortex_m::CortexM;
use crate::programmer::swd::SwdTransport;

pub const RTT_SIGNATURE: &[u8; 16] = b"SEGGER RTT\0\0\0\0\0\0";
pub const RTT_SIGNATURE_LEN: usize = 16;
pub const DEFAULT_SRAM_START: u32 = 0x20000000;
pub const DEFAULT_SRAM_SIZE: u32 = 0x10000; // 64 KB

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RttError {
    NotFound,
    SwdError,
    BufferCorrupt,
    TargetNotResponding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RttBufferDesc {
    pub name_ptr: u32,
    pub buffer_ptr: u32,
    pub size: u32,
    pub write_offset: u32,
    pub read_offset: u32,
    pub flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RttControlBlock {
    pub cb_addr: u32,
    pub max_num_up_buffers: u32,
    pub max_num_down_buffers: u32,
    pub up_buffer: RttBufferDesc,
}

impl RttControlBlock {
    pub fn parse_from_slice(cb_addr: u32, data: &[u8]) -> Option<Self> {
        if data.len() < 48 {
            return None;
        }

        // Validate signature: must match "SEGGER RTT" in the first 10 bytes
        if &data[0..10] != b"SEGGER RTT" {
            return None;
        }

        let max_num_up = u32::from_le_bytes(data[16..20].try_into().ok()?);
        let max_num_down = u32::from_le_bytes(data[20..24].try_into().ok()?);

        if max_num_up == 0 || max_num_up > 32 || max_num_down > 32 {
            return None;
        }

        // Up Buffer 0 at offset 24
        let name_ptr = u32::from_le_bytes(data[24..28].try_into().ok()?);
        let buffer_ptr = u32::from_le_bytes(data[28..32].try_into().ok()?);
        let size = u32::from_le_bytes(data[32..36].try_into().ok()?);
        let write_offset = u32::from_le_bytes(data[36..40].try_into().ok()?);
        let read_offset = u32::from_le_bytes(data[40..44].try_into().ok()?);
        let flags = u32::from_le_bytes(data[44..48].try_into().ok()?);

        if size == 0 || size > 65536 || buffer_ptr == 0 {
            return None;
        }

        Some(Self {
            cb_addr,
            max_num_up_buffers: max_num_up,
            max_num_down_buffers: max_num_down,
            up_buffer: RttBufferDesc {
                name_ptr,
                buffer_ptr,
                size,
                write_offset,
                read_offset,
                flags,
            },
        })
    }
}

pub struct RttEngineState {
    pub found: bool,
    pub cb: RttControlBlock,
    pub total_bytes_read: u32,
    pub last_read_len: u32,
}

impl RttEngineState {
    pub const fn new() -> Self {
        Self {
            found: false,
            cb: RttControlBlock {
                cb_addr: 0,
                max_num_up_buffers: 0,
                max_num_down_buffers: 0,
                up_buffer: RttBufferDesc {
                    name_ptr: 0,
                    buffer_ptr: 0,
                    size: 0,
                    write_offset: 0,
                    read_offset: 0,
                    flags: 0,
                },
            },
            total_bytes_read: 0,
            last_read_len: 0,
        }
    }
}

static RTT_ENGINE: Mutex<CriticalSectionRawMutex, RefCell<RttEngineState>> =
    Mutex::new(RefCell::new(RttEngineState::new()));

// Ring buffer for storing recent log lines for WebUI / SSE / WebSocket clients
static RTT_LOG_BUFFER: Mutex<CriticalSectionRawMutex, RefCell<heapless::String<2048>>> =
    Mutex::new(RefCell::new(heapless::String::new()));

/// Scans target SRAM for the SEGGER RTT control block without halting target core
pub fn scan_rtt(
    cortex: &mut CortexM,
    sram_start: u32,
    sram_size: u32,
) -> Result<RttControlBlock, RttError> {
    let mut block_buf = [0u8; 256];
    let end_addr = sram_start.saturating_add(sram_size);
    let mut curr_addr = sram_start;

    while curr_addr < end_addr {
        let read_len = ((end_addr - curr_addr) as usize).min(block_buf.len());
        if cortex.mem_read_block(curr_addr, &mut block_buf[..read_len]).is_err() {
            curr_addr += 256;
            continue;
        }

        // Search for "SEGGER RTT" signature within this block
        if read_len >= 16 {
            for i in (0..=(read_len - 16)).step_by(4) {
                if &block_buf[i..i + 10] == b"SEGGER RTT" {
                    let candidate_addr = curr_addr + i as u32;
                    let mut cb_header = [0u8; 48];
                    if cortex.mem_read_block(candidate_addr, &mut cb_header).is_ok() {
                        if let Some(cb) = RttControlBlock::parse_from_slice(candidate_addr, &cb_header) {
                            return Ok(cb);
                        }
                    }
                }
            }
        }
        curr_addr += 256;
    }

    Err(RttError::NotFound)
}

/// Drains new bytes from Up Buffer 0 and updates target read pointer
pub fn drain_rtt_up(
    cortex: &mut CortexM,
    cb: &mut RttControlBlock,
    out: &mut [u8],
) -> Result<usize, RttError> {
    let size = cb.up_buffer.size;
    if size == 0 {
        return Ok(0);
    }

    // Read current write pointer from target RAM (offset 36 in control block)
    let wr_off = cortex
        .mem_read32(cb.cb_addr + 24 + 12)
        .map_err(|_| RttError::SwdError)?
        % size;

    // Read current read pointer from target RAM (offset 40 in control block)
    let rd_off = cortex
        .mem_read32(cb.cb_addr + 24 + 16)
        .map_err(|_| RttError::SwdError)?
        % size;

    if wr_off == rd_off {
        return Ok(0);
    }

    let mut bytes_drained = 0usize;

    if wr_off > rd_off {
        let avail = (wr_off - rd_off) as usize;
        let to_read = avail.min(out.len());
        cortex
            .mem_read_block(cb.up_buffer.buffer_ptr + rd_off, &mut out[..to_read])
            .map_err(|_| RttError::SwdError)?;
        bytes_drained = to_read;
        let new_rd = (rd_off + to_read as u32) % size;
        cortex
            .mem_write32(cb.cb_addr + 24 + 16, new_rd)
            .map_err(|_| RttError::SwdError)?;
        cb.up_buffer.read_offset = new_rd;
    } else {
        // Buffer wrap-around
        let part1 = (size - rd_off) as usize;
        let to_read1 = part1.min(out.len());
        cortex
            .mem_read_block(cb.up_buffer.buffer_ptr + rd_off, &mut out[..to_read1])
            .map_err(|_| RttError::SwdError)?;
        bytes_drained += to_read1;

        if to_read1 == part1 && bytes_drained < out.len() {
            let part2 = wr_off as usize;
            let to_read2 = part2.min(out.len() - bytes_drained);
            if to_read2 > 0 {
                cortex
                    .mem_read_block(cb.up_buffer.buffer_ptr, &mut out[bytes_drained..bytes_drained + to_read2])
                    .map_err(|_| RttError::SwdError)?;
                bytes_drained += to_read2;
            }
            let new_rd = to_read2 as u32;
            cortex
                .mem_write32(cb.cb_addr + 24 + 16, new_rd)
                .map_err(|_| RttError::SwdError)?;
            cb.up_buffer.read_offset = new_rd;
        } else {
            let new_rd = (rd_off + to_read1 as u32) % size;
            cortex
                .mem_write32(cb.cb_addr + 24 + 16, new_rd)
                .map_err(|_| RttError::SwdError)?;
            cb.up_buffer.read_offset = new_rd;
        }
    }

    cb.up_buffer.write_offset = wr_off;
    Ok(bytes_drained)
}

/// Global scan helper
pub fn rtt_scan_global() -> Result<RttControlBlock, RttError> {
    let mut swd = SwdTransport::new();
    swd.init(4000);
    swd.jtag_to_swd();
    let mut dpidr = 0u32;
    let _ = swd.read_dpidr_retry(&mut dpidr);
    let _ = swd.power_up_debug();

    let mut cortex = CortexM::new(&mut swd);
    let cb = scan_rtt(&mut cortex, DEFAULT_SRAM_START, DEFAULT_SRAM_SIZE)?;

    RTT_ENGINE.lock(|engine| {
        let mut state = engine.borrow_mut();
        state.found = true;
        state.cb = cb;
    });

    Ok(cb)
}

/// Global drain helper
pub fn rtt_drain_global(out: &mut [u8]) -> Result<usize, RttError> {
    let mut cb = RTT_ENGINE.lock(|engine| {
        let state = engine.borrow();
        if !state.found {
            None
        } else {
            Some(state.cb)
        }
    });

    let mut cb = match cb {
        Some(c) => c,
        None => {
            // Auto scan if not yet discovered
            rtt_scan_global()?
        }
    };

    let mut swd = SwdTransport::new();
    swd.init(4000);
    swd.jtag_to_swd();
    let mut dpidr = 0u32;
    let _ = swd.read_dpidr_retry(&mut dpidr);
    let _ = swd.power_up_debug();

    let mut cortex = CortexM::new(&mut swd);
    let n = drain_rtt_up(&mut cortex, &mut cb, out)?;

    if n > 0 {
        RTT_ENGINE.lock(|engine| {
            let mut state = engine.borrow_mut();
            state.cb = cb;
            state.total_bytes_read = state.total_bytes_read.saturating_add(n as u32);
            state.last_read_len = n as u32;
        });

        // Append to log ring buffer
        if let Ok(s) = core::str::from_utf8(&out[..n]) {
            RTT_LOG_BUFFER.lock(|log| {
                let mut l = log.borrow_mut();
                if l.len() + s.len() > l.capacity() {
                    l.clear();
                }
                let _ = l.push_str(s);
            });
        }
    }

    Ok(n)
}

/// Status JSON formatter
pub fn rtt_status_json(buf: &mut heapless::String<256>) {
    RTT_ENGINE.lock(|engine| {
        let state = engine.borrow();
        if state.found {
            let _ = core::write!(
                buf,
                r#"{{"status":"ok","found":true,"cb_addr":"0x{:08X}","buffer_size":{},"buffer_ptr":"0x{:08X}","total_bytes_read":{}}}"#,
                state.cb.cb_addr,
                state.cb.up_buffer.size,
                state.cb.up_buffer.buffer_ptr,
                state.total_bytes_read
            );
        } else {
            let _ = core::write!(
                buf,
                r#"{{"status":"ok","found":false,"message":"RTT control block not found"}}"#
            );
        }
    });
}
