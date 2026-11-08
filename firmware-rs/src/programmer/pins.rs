use super::rvswd::RvswdTransport;
use super::swd::SwdTransport;
use super::swio::SwioTransport;
use crate::board::ActiveBoard;
use agentla_bsp::BoardProfile;
use rp_pac as pac;

pub const PIN_SWCLK: usize = ActiveBoard::PROGRAMMER.swclk as usize;
pub const PIN_SWDIO: usize = ActiveBoard::PROGRAMMER.swdio as usize;
pub const PIN_RESET: usize = ActiveBoard::PROGRAMMER.nrst as usize;
pub const PIN_BOOT0: usize = ActiveBoard::PROGRAMMER.boot0 as usize;
pub const PIN_UART_TX: usize = ActiveBoard::PROGRAMMER.uart_tx as usize;
pub const PIN_UART_RX: usize = ActiveBoard::PROGRAMMER.uart_rx as usize;

/// Programmer-bridge pin block (fixed programmatic resources).
/// Single source of truth: `tristate_pins` below and the fixed claims in
/// `crate::pin_owner` are seeded from this list.
pub const BRIDGE_PINS: [usize; 4] = [
    ActiveBoard::PROGRAMMER.swclk as usize,
    ActiveBoard::PROGRAMMER.swdio as usize,
    ActiveBoard::PROGRAMMER.nrst as usize,
    ActiveBoard::PROGRAMMER.boot0 as usize,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetPort {
    BluePill, // GPIO 4 (SWCLK), 5 (SWDIO), 6 (nRST)
    Ch32v203, // GPIO 8 (SWCLK), 9 (SWDIO), 10 (nRST)
    Ch32v003, // GPIO 12 (SWIO), 13 (nRST), 14 (UART RX)
}

/// All pins used across all 3 target ports
pub const ALL_TARGET_PINS: [usize; 9] = [4, 5, 6, 8, 9, 10, 12, 13, 14];

/// Tristate and safely float all programmer pins (all ports, GPIO 4–14)
pub fn tristate_pins() {
    for pin in ALL_TARGET_PINS {
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
            TargetPort::BluePill => transport.init_pins(PIN_SWCLK, PIN_SWDIO, PIN_RESET, freq_khz),
            TargetPort::Ch32v203 => transport.init_pins(8, 9, 10, freq_khz),
            TargetPort::Ch32v003 => transport.init_pins(PIN_SWCLK, PIN_SWDIO, PIN_RESET, freq_khz),
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
            TargetPort::Ch32v003 => {
                drive_reset_pin(13, false);
                transport.init_pin(12);
            }
            TargetPort::Ch32v203 => {
                drive_reset_pin(10, false);
                transport.init_pin(9);
            }
            TargetPort::BluePill => {
                drive_reset_pin(PIN_RESET, false);
                transport.init_pin(PIN_SWDIO);
            }
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
            TargetPort::Ch32v203 => {
                drive_reset_pin(10, false);
                transport.init_pins(8, 9);
            }
            TargetPort::BluePill => {
                drive_reset_pin(PIN_RESET, false);
                transport.init_pins(PIN_SWCLK, PIN_SWDIO);
            }
            TargetPort::Ch32v003 => {
                drive_reset_pin(13, false);
                transport.init_pins(PIN_SWCLK, PIN_SWDIO);
            }
        }
        RvswdSession {
            transport,
            _pins: self,
        }
    }

    pub fn set_reset(&mut self, assert_low: bool) {
        drive_reset_pin(PIN_RESET, assert_low);
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

pub fn drive_reset_pin(pin: usize, assert_low: bool) {
    if assert_low {
        // Drive low
        pac::PADS_BANK0.gpio(pin).modify(|w| {
            w.set_iso(false);
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
    } else {
        // Release high (input with pullup)
        pac::PADS_BANK0.gpio(pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
        pac::IO_BANK0.gpio(pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
    }
}

pub fn pulse_reset_pin(pin: usize) {
    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_iso(false);
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
        w.set_iso(false);
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

    pub fn connect_under_reset(&mut self) -> Result<u32, crate::programmer::target::cortex_m::CortexError> {
        let mut cortex = crate::programmer::target::cortex_m::CortexM::new(&mut self.transport);
        cortex.connect_under_reset()
    }

    pub fn reset_and_catch(&mut self) -> Result<u32, crate::programmer::target::cortex_m::CortexError> {
        let mut cortex = crate::programmer::target::cortex_m::CortexM::new(&mut self.transport);
        cortex.reset_and_catch()
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
        // Retry loop matching minichlink DefaultSetupInterface / DefaultUnbrick
        for _ in 0..50 {
            self.transport.put(0x7E, 0x5AA50400); // DMSHDWCFGR
            self.transport.put(0x7D, 0x5AA50400); // DMCFGR
            self.transport.put(0x10, 0x80000001); // DMCONTROL haltreq + dmactive
            self.transport.put(0x10, 0x80000001);
            if let Ok(dmstatus) = self.transport.get(0x11) {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    return Ok(dmstatus);
                }
            }
            cortex_m::asm::delay(30_000); // ~200 µs
        }

        // Pulse nRESET on GPIO 13, wait 15ms recovery, re-initialize SWIO pin
        pulse_reset_pin(13);
        cortex_m::asm::delay(2_250_000); // 15 ms
        self.transport.init_pin(12);

        for _ in 0..100 {
            self.transport.put(0x7E, 0x5AA50400);
            self.transport.put(0x7D, 0x5AA50400);
            self.transport.put(0x10, 0x80000001);
            self.transport.put(0x10, 0x80000001);
            if let Ok(dmstatus) = self.transport.get(0x11) {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    return Ok(dmstatus);
                }
            }
            cortex_m::asm::delay(30_000);
        }

        Err(crate::programmer::swio::SwioError::Timeout)
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
        let _ = self.transport.reset();
        for _ in 0..10 {
            if let Ok(dmstatus) = self.transport.read(0x11) {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    return Ok(dmstatus);
                }
            }
            let _ = self.transport.reset();
            cortex_m::asm::delay(30_000);
        }
        // Pulse nRESET on GPIO 10 and retry
        pulse_reset_pin(10);
        let _ = self.transport.reset();
        for _ in 0..10 {
            if let Ok(dmstatus) = self.transport.read(0x11) {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    return Ok(dmstatus);
                }
            }
            cortex_m::asm::delay(30_000);
        }
        Err(crate::programmer::rvswd::RvswdError::Timeout)
    }

    pub fn release(mut self) -> ProgrammerPins {
        self.transport.deinit();
        tristate_pins();
        self._pins
    }
}
