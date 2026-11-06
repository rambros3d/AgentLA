#![allow(dead_code, unused)]
//! Hardware & bit-banged SPI master bridge.
//!
//! Provides bidirectional multi-byte SPI master transfers matching RP2350 SPI0
//! pinouts (MISO: GPIO 16, CS: GPIO 17, SCK: GPIO 18, MOSI: GPIO 19) or flexible
//! user-assigned GPIO pins. Supports SPI modes 0, 1, 2, 3 and variable baud rates.

use crate::board::ActiveBoard;
use crate::pin_owner;
use agentla_bsp::BoardProfile;
use rp_pac as pac;

/// Default hardware SPI pins from active board profile.
pub const PIN_MISO: u32 = ActiveBoard::ADAPTERS.default_spi.miso;
pub const PIN_CS: u32 = ActiveBoard::ADAPTERS.default_spi.cs;
pub const PIN_SCK: u32 = ActiveBoard::ADAPTERS.default_spi.sck;
pub const PIN_MOSI: u32 = ActiveBoard::ADAPTERS.default_spi.mosi;
pub const SPI_PINS: [u32; 4] = [PIN_MISO, PIN_CS, PIN_SCK, PIN_MOSI];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpiMode {
    Mode0 = 0, // CPOL=0, CPHA=0
    Mode1 = 1, // CPOL=0, CPHA=1
    Mode2 = 2, // CPOL=1, CPHA=0
    Mode3 = 3, // CPOL=1, CPHA=1
}

pub struct SpiBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub mode: u8,
    pub speed_hz: u32,
    tx_bytes: u32,
    rx_bytes: u32,
}

impl SpiBridge {
    pub fn new() -> Self {
        Self {
            active: false,
            slot: None,
            mode: 0,
            speed_hz: 1_000_000,
            tx_bytes: 0,
            rx_bytes: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }
    pub fn tx_count(&self) -> u32 {
        self.tx_bytes
    }
    pub fn rx_count(&self) -> u32 {
        self.rx_bytes
    }

    pub fn safe_pins() -> &'static [u32] {
        &SPI_PINS
    }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active {
            return Ok(());
        }
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &SPI_PINS, pin_owner::CAP_SPI)?;
        self.slot = Some(slot);
        self.active = true;

        init_spi_gpios(PIN_SCK, PIN_MOSI, PIN_MISO, PIN_CS, self.mode);
        Ok(())
    }

    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() {
            let _ = pin_owner::release(slot);
        }
        self.active = false;
    }
}

fn init_spi_gpios(sck: u32, mosi: u32, miso: u32, cs: u32, mode: u8) {
    let cpol = (mode & 2) != 0;
    unsafe {
        for &pin in &[sck, mosi, cs] {
            pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                w.set_funcsel(5); // SIO
            });
            pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                w.set_iso(false);
                w.set_ie(false);
                w.set_od(false);
                w.set_pue(false);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
        }

        // Configure MISO as input with pullup
        pac::IO_BANK0.gpio(miso as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(miso as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << miso);

        // CS idle high
        pac::SIO.gpio_out(0).value_set().write_value(1 << cs);

        // SCK idle level depends on CPOL
        if cpol {
            pac::SIO.gpio_out(0).value_set().write_value(1 << sck);
        } else {
            pac::SIO.gpio_out(0).value_clr().write_value(1 << sck);
        }
    }
}

fn delay_cycles(cycles: u32) {
    cortex_m::asm::delay(cycles.max(2));
}

/// Perform a full-duplex SPI transfer across SCK/MOSI/MISO/CS.
pub fn hw_spi_transfer(
    sck: u32,
    mosi: u32,
    miso: u32,
    cs: u32,
    mode: u8,
    speed_hz: u32,
    tx_data: &[u8],
    rx_data: &mut [u8],
) -> usize {
    init_spi_gpios(sck, mosi, miso, cs, mode);

    let cpol = (mode & 2) != 0;
    let cpha = (mode & 1) != 0;

    // RP2350 CPU runs at 150 MHz. Calculate half-period delay in cycles.
    // e.g. 1 MHz => 75 cycles per half clock.
    let speed = speed_hz.clamp(1_000, 10_000_000);
    let half_period_cycles = (150_000_000 / (speed * 2)).max(2);

    let len = tx_data.len().max(rx_data.len());
    if len == 0 {
        return 0;
    }

    unsafe {
        // Assert CS (drive low)
        pac::SIO.gpio_out(0).value_clr().write_value(1 << cs);
        delay_cycles(half_period_cycles);

        for i in 0..len {
            let tx_byte = if i < tx_data.len() { tx_data[i] } else { 0xFF };
            let mut rx_byte = 0u8;

            for bit in (0..8).rev() {
                let mosi_bit = (tx_byte >> bit) & 1;

                if !cpha {
                    // CPHA=0: MOSI setup before leading edge
                    if mosi_bit != 0 {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << mosi);
                    } else {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << mosi);
                    }
                    delay_cycles(half_period_cycles);

                    // Leading edge
                    if cpol {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << sck);
                    } else {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << sck);
                    }

                    // Sample MISO on leading edge
                    let miso_val = (pac::SIO.gpio_in(0).read() & (1 << miso)) != 0;
                    if miso_val {
                        rx_byte |= 1 << bit;
                    }
                    delay_cycles(half_period_cycles);

                    // Trailing edge
                    if cpol {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << sck);
                    } else {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << sck);
                    }
                } else {
                    // CPHA=1: Leading edge first
                    if cpol {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << sck);
                    } else {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << sck);
                    }

                    // MOSI setup after leading edge
                    if mosi_bit != 0 {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << mosi);
                    } else {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << mosi);
                    }
                    delay_cycles(half_period_cycles);

                    // Trailing edge
                    if cpol {
                        pac::SIO.gpio_out(0).value_set().write_value(1 << sck);
                    } else {
                        pac::SIO.gpio_out(0).value_clr().write_value(1 << sck);
                    }

                    // Sample MISO on trailing edge
                    let miso_val = (pac::SIO.gpio_in(0).read() & (1 << miso)) != 0;
                    if miso_val {
                        rx_byte |= 1 << bit;
                    }
                    delay_cycles(half_period_cycles);
                }
            }

            if i < rx_data.len() {
                rx_data[i] = rx_byte;
            }
        }

        // Deassert CS (drive high)
        delay_cycles(half_period_cycles);
        pac::SIO.gpio_out(0).value_set().write_value(1 << cs);
        delay_cycles(half_period_cycles);
    }

    len
}

/// Convenience transfer using default SPI0 pins
pub fn spi_transfer_default(mode: u8, speed_hz: u32, tx: &[u8], rx: &mut [u8]) -> usize {
    hw_spi_transfer(PIN_SCK, PIN_MOSI, PIN_MISO, PIN_CS, mode, speed_hz, tx, rx)
}
