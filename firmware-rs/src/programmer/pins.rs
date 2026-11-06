use super::rvswd::RvswdTransport;
use super::swd::SwdTransport;
use super::swio::SwioTransport;
use rp_pac as pac;

pub const PIN_SWCLK: usize = 4;
pub const PIN_SWDIO: usize = 5;
pub const PIN_RESET: usize = 6;
pub const PIN_BOOT0: usize = 7;

/// Programmer-bridge pin block (fixed programmatic resources GPIO 4–9).
/// Single source of truth: `tristate_pins` below and the fixed claims in
/// `crate::pin_owner` are seeded from this list.
pub const BRIDGE_PINS: [usize; 6] = [4, 5, 6, 7, 8, 9];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPort {
    BluePill, // GPIO 4 (SWCLK), 5 (SWDIO), 6 (nRST)
    Ch32v203, // GPIO 8 (SWCLK), 9 (SWDIO), 10 (nRST)
    Ch32v003, // GPIO 12 (SWIO), 13 (nRST), 14 (UART RX)
}

/// Tristate and safely float all programmer pins (bridge block, GPIO 4–14 excluding 11)
pub fn tristate_pins() {
    for pin in BRIDGE_PINS {
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
        pac::PADS_BANK0.gpio(pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true); // output disabled = input (High-Z)
            w.set_pue(true); // weak pull-up
            w.set_pde(false);
        });
        pac::IO_BANK0.gpio(pin).ctrl().write(|w| {
            w.set_funcsel(5); // SIO (SIOB_PROC_0) so gpio_in reads live voltages
        });
    }
}

#[derive(Clone, Copy)]
pub struct ProgrammerPins {
    _private: (),
}

impl ProgrammerPins {
    pub const fn new() -> Self {
        Self { _private: () }
    }

    pub fn into_swd(self, freq_khz: u32) -> SwdSession {
        self.into_swd_port(TargetPort::BluePill, freq_khz)
    }

    pub fn into_swd_port(self, port: TargetPort, freq_khz: u32) -> SwdSession {
        tristate_pins();
        let mut transport = SwdTransport::new();
        match port {
            TargetPort::BluePill => transport.init_pins(4, 5, 6, freq_khz),
            TargetPort::Ch32v203 => transport.init_pins(8, 9, 10, freq_khz),
            TargetPort::Ch32v003 => transport.init_pins(4, 5, 6, freq_khz),
        }
        SwdSession {
            transport,
            _pins: self,
        }
    }

    pub fn into_swio(self) -> SwioSession {
        self.into_swio_port(TargetPort::Ch32v003)
    }

    pub fn into_swio_port(self, port: TargetPort) -> SwioSession {
        tristate_pins();
        let mut transport = SwioTransport::new();
        match port {
            TargetPort::Ch32v003 => transport.init_pin(12),
            TargetPort::Ch32v203 => transport.init_pin(9),
            TargetPort::BluePill => transport.init_pin(5),
        }
        SwioSession {
            transport,
            _pins: self,
        }
    }

    pub fn into_rvswd(self) -> RvswdSession {
        self.into_rvswd_port(TargetPort::Ch32v203)
    }

    pub fn into_rvswd_port(self, port: TargetPort) -> RvswdSession {
        tristate_pins();
        let mut transport = RvswdTransport::new();
        match port {
            TargetPort::Ch32v203 => transport.init_pins(8, 9),
            TargetPort::BluePill => transport.init_pins(4, 5),
            TargetPort::Ch32v003 => transport.init_pins(4, 5),
        }
        RvswdSession {
            transport,
            _pins: self,
        }
    }

    pub fn set_reset(&mut self, assert_low: bool) {
        if assert_low {
            // Drive low
            pac::PADS_BANK0.gpio(PIN_RESET).modify(|w| {
                w.set_ie(true);
                w.set_od(false);
                w.set_pue(false);
                w.set_pde(false);
            });
            pac::IO_BANK0.gpio(PIN_RESET).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
            });
            pac::SIO.gpio_out(0).value_clr().write_value(1 << PIN_RESET);
            pac::SIO.gpio_oe(0).value_set().write_value(1 << PIN_RESET);
        } else {
            // Release high (input with pullup)
            pac::PADS_BANK0.gpio(PIN_RESET).modify(|w| {
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
                w.set_pde(false);
            });
            pac::SIO.gpio_oe(0).value_clr().write_value(1 << PIN_RESET);
            pac::IO_BANK0.gpio(PIN_RESET).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
            });
        }
    }

    pub fn read_reset(&self) -> bool {
        (pac::SIO.gpio_in(0).read() & (1 << PIN_RESET)) != 0
    }

    pub fn set_boot0(&mut self, high: bool) {
        pac::PADS_BANK0.gpio(PIN_BOOT0).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::IO_BANK0.gpio(PIN_BOOT0).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << PIN_BOOT0);
        if high {
            pac::SIO.gpio_out(0).value_set().write_value(1 << PIN_BOOT0);
        } else {
            pac::SIO.gpio_out(0).value_clr().write_value(1 << PIN_BOOT0);
        }
    }
}

pub struct SwdSession {
    pub transport: SwdTransport,
    _pins: ProgrammerPins,
}

impl core::ops::Deref for SwdSession {
    type Target = SwdTransport;
    fn deref(&self) -> &Self::Target {
        &self.transport
    }
}

impl core::ops::DerefMut for SwdSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transport
    }
}

impl Drop for SwdSession {
    fn drop(&mut self) {
        self.transport.deinit();
        tristate_pins();
    }
}

pub fn pulse_reset_pin(pin: usize) {
    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_ie(true);
        w.set_od(false);
        w.set_pue(false);
        w.set_pde(false);
    });
    pac::IO_BANK0.gpio(pin).ctrl().write(|w| {
        w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
    });
    pac::SIO.gpio_out(0).value_clr().write_value(1 << pin);
    pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
    cortex_m::asm::delay(3_000_000); // 20 ms
    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_ie(true);
        w.set_od(true);
        w.set_pue(true);
        w.set_pde(false);
    });
    pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    cortex_m::asm::delay(7_500_000); // 50 ms
}

impl SwdSession {
    pub fn attach(&mut self) -> Result<u32, crate::programmer::swd::SwdError> {
        self.transport.jtag_to_swd();
        let mut dpidr = 0u32;
        if self.transport.read_dpidr_retry(&mut dpidr).is_ok() && dpidr != 0 && dpidr != 0xFFFFFFFF {
            self.transport.power_up_debug()?;
            return Ok(dpidr);
        }
        // Pulse nRESET: assert low 20ms, release high, wait 20ms recovery
        let pin = self.transport.reset_pin;
        pac::PADS_BANK0.gpio(pin).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::IO_BANK0.gpio(pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::SIO.gpio_out(0).value_clr().write_value(1 << pin);
        pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
        cortex_m::asm::delay(3_000_000); // 20 ms low

        // Release nRESET
        pac::PADS_BANK0.gpio(pin).modify(|w| {
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
        cortex_m::asm::delay(3_000_000); // 20 ms recovery

        self.transport.jtag_to_swd();
        let res = self.transport.read_dpidr_retry(&mut dpidr);
        res?;
        self.transport.power_up_debug()?;
        Ok(dpidr)
    }

    pub fn release(mut self) -> ProgrammerPins {
        self.transport.deinit();
        tristate_pins();
        self._pins
    }
}

pub struct SwioSession {
    pub transport: SwioTransport,
    _pins: ProgrammerPins,
}

impl core::ops::Deref for SwioSession {
    type Target = SwioTransport;
    fn deref(&self) -> &Self::Target {
        &self.transport
    }
}

impl core::ops::DerefMut for SwioSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transport
    }
}

impl Drop for SwioSession {
    fn drop(&mut self) {
        self.transport.deinit();
        tristate_pins();
    }
}

impl SwioSession {
    pub fn attach(&mut self) -> Result<u32, crate::programmer::swio::SwioError> {
        // Halt and reset sequence matching proven PicoRVD / minichlink:
        self.transport.put(0x10, 0x80000001); // Halt core
        cortex_m::asm::delay(150_000);
        self.transport.put(0x10, 0x80000003); // Assert NDMRESET
        cortex_m::asm::delay(300_000);
        self.transport.put(0x10, 0x80000001); // Release NDMRESET while holding halt
        cortex_m::asm::delay(300_000);
        self.transport.put(0x10, 0x90000001); // Ack havereset (bit 28)
        cortex_m::asm::delay(150_000);
        self.transport.put(0x10, 0x80000001); // Keep core halted
        self.transport.put(0x18, 0x00000000); // Clear ABSTRACTAUTO
        self.transport.put(0x16, 0x08000700); // Clear ABSTRACTCS errors (cmderr)
        cortex_m::asm::delay(150_000);

        let mut dmstatus = self.transport.get(0x11).unwrap_or(0);
        if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
            return Ok(dmstatus);
        }
        // Pulse nRESET on GPIO 13 and retry
        pulse_reset_pin(13);
        self.transport.init_pin(12);
        self.transport.put(0x10, 0x80000001);
        self.transport.put(0x18, 0x00000000);
        self.transport.put(0x16, 0x08000700);
        dmstatus = self.transport.get(0x11)?;
        if dmstatus == 0 || dmstatus == 0xFFFFFFFF {
            return Err(crate::programmer::swio::SwioError::Timeout);
        }
        Ok(dmstatus)
    }

    pub fn release(mut self) -> ProgrammerPins {
        self.transport.deinit();
        tristate_pins();
        self._pins
    }
}

pub struct RvswdSession {
    pub transport: RvswdTransport,
    _pins: ProgrammerPins,
}

impl core::ops::Deref for RvswdSession {
    type Target = RvswdTransport;
    fn deref(&self) -> &Self::Target {
        &self.transport
    }
}

impl core::ops::DerefMut for RvswdSession {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transport
    }
}

impl Drop for RvswdSession {
    fn drop(&mut self) {
        self.transport.deinit();
        tristate_pins();
    }
}

impl RvswdSession {
    pub fn attach(&mut self) -> Result<u32, crate::programmer::rvswd::RvswdError> {
        let mut dmstatus = self.transport.read(0x11).unwrap_or(0);
        if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
            return Ok(dmstatus);
        }
        let _ = self.transport.reset();
        dmstatus = self.transport.read(0x11).unwrap_or(0);
        if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
            return Ok(dmstatus);
        }
        // Pulse nRESET on GPIO 10 and retry
        pulse_reset_pin(10);
        let _ = self.transport.reset();
        dmstatus = self.transport.read(0x11)?;
        if dmstatus == 0 || dmstatus == 0xFFFFFFFF {
            return Err(crate::programmer::rvswd::RvswdError::Timeout);
        }
        Ok(dmstatus)
    }

    pub fn release(mut self) -> ProgrammerPins {
        self.transport.deinit();
        tristate_pins();
        self._pins
    }
}
