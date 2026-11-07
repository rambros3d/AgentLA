use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;

pub use agentla_debug_server::rtt::{
    format_rtt_status_json, RttBufferDesc, RttControlBlock, RttEngineState, RttError,
    DEFAULT_SRAM_SIZE, DEFAULT_SRAM_START, RTT_SIGNATURE, RTT_SIGNATURE_LEN,
};

use crate::programmer::swd::SwdTransport;
use crate::programmer::target::cortex_m::{CortexM, CortexTarget};

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
    let mut target = CortexTarget(cortex);
    agentla_debug_server::rtt::scan_rtt(&mut target, sram_start, sram_size)
}

/// Drains new bytes from Up Buffer 0 and updates target read pointer
pub fn drain_rtt_up(
    cortex: &mut CortexM,
    cb: &mut RttControlBlock,
    out: &mut [u8],
) -> Result<usize, RttError> {
    let mut target = CortexTarget(cortex);
    agentla_debug_server::rtt::drain_rtt_up(&mut target, cb, out)
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
    let cb = RTT_ENGINE.lock(|engine| {
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
        format_rtt_status_json(&state, buf);
    });
}
