//! Serial Wire Output (SWO) hardware receiver driver.
//!
//! Captures asynchronous NRZ UART trace bytes on the board's designated SWO pin
//! (GPIO 9 on Pico 2, GPIO 17 on BakuProbe) and feeds them into `SWO_ENGINE`.

use crate::board::ActiveBoard;
use agentla_bsp::BoardProfile;
use agentla_programmer::swo::SwoEngine;
use rp_pac as pac;

/// Global shared SWO trace capture engine.
pub static SWO_ENGINE: SwoEngine = SwoEngine::new();

/// Designated hardware SWO pin from active board profile.
pub const PIN_SWO: u32 = match ActiveBoard::PROGRAMMER.tdo {
    Some(pin) => pin,
    None => ActiveBoard::PROGRAMMER.uart_rx,
};

/// Initialize SWO GPIO pad as digital input with pull-up.
pub fn init_swo_pin(pin: u32) {
    unsafe {
        pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
            w.set_funcsel(5); // SIO input
        });
        pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    }
}

#[inline(always)]
fn delay_cycles(cycles: u32) {
    cortex_m::asm::delay(cycles.max(2));
}

/// Sample a single UART NRZ byte on `pin` at `baud`.
/// Returns `Some(byte)` if a valid start bit and frame were received, or `None` if idle high.
pub fn poll_swo_rx(pin: u32, baud: u32) -> Option<u8> {
    let sys_clk = crate::benchmark::get_sys_clk_hz();
    let bit_cycles = sys_clk / baud.clamp(9600, 4_500_000);

    unsafe {
        // Line is idle high; check for start bit (low)
        if (pac::SIO.gpio_in(0).read() & (1 << pin)) != 0 {
            return None;
        }

        // Wait 1.5 bit periods to sample at the center of bit 0
        delay_cycles(bit_cycles + (bit_cycles / 2));

        let mut byte = 0u8;
        for i in 0..8 {
            if (pac::SIO.gpio_in(0).read() & (1 << pin)) != 0 {
                byte |= 1 << i;
            }
            delay_cycles(bit_cycles);
        }

        // Delay to middle of stop bit
        delay_cycles(bit_cycles / 2);

        Some(byte)
    }
}

/// Poll a burst of incoming SWO bytes into `SWO_ENGINE`.
pub fn poll_swo_burst(pin: u32, baud: u32, max_bytes: usize) -> usize {
    let mut count = 0;
    while count < max_bytes {
        if let Some(b) = poll_swo_rx(pin, baud) {
            SWO_ENGINE.push_byte(b);
            count += 1;
        } else {
            break;
        }
    }
    count
}
