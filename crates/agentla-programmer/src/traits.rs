//! Hardware-independent transport traits for debug protocols.

use core::fmt::Debug;

pub trait SwdIo {
    type Error: Debug;

    /// Execute an SWD transfer.
    /// `req`: 8-bit SWD request packet (Start, APnDP, RnW, A[2:3], Parity, Stop, Park).
    /// `data`: `Some(val)` for write transfers, `None` for read transfers.
    /// Returns `(ack, read_data)` where `ack` is the 3-bit response (1=OK, 2=WAIT, 4=FAULT).
    fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error>;

    /// Line reset sequence (e.g. >= 50 high clocks followed by idle clocks).
    fn reset_sequence(&mut self) -> Result<(), Self::Error>;

    /// Optional raw bit write (default no-op).
    fn write_bits(&mut self, _bit_count: u32, _data: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Optional raw bit read (default return 0).
    fn read_bits(&mut self, _bit_count: u32) -> Result<u32, Self::Error> {
        Ok(0)
    }

    /// JTAG-to-SWD sequence.
    fn jtag_to_swd(&mut self) -> Result<(), Self::Error> {
        self.reset_sequence()
    }

    /// Dormant-to-SWD sequence.
    fn dormant_to_swd(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Set clock frequency in kHz.
    fn set_clock(&mut self, _freq_khz: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Set turnaround cycles.
    fn set_turnaround(&mut self, _cycles: u8) {}

    /// Set idle cycles.
    fn set_idle_cycles(&mut self, _cycles: u8) {}

    /// Assert or deassert hardware reset line (true = assert/pull low, false = release/float).
    fn assert_reset(&mut self, _assert: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Direct SWJ pin manipulation for CMSIS-DAP DAP_SWJ_Pins command.
    /// Returns current state of pins (Bit 0: SWCLK, Bit 1: SWDIO, Bit 7: nRESET).
    fn swj_pins(
        &mut self,
        _pin_output: u8,
        _pin_select: u8,
        _wait_us: u32,
    ) -> Result<u8, Self::Error> {
        Ok(0)
    }
}

impl<T: SwdIo + ?Sized> SwdIo for &mut T {
    type Error = T::Error;
    fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error> {
        (**self).transfer(req, data)
    }
    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        (**self).reset_sequence()
    }
    fn write_bits(&mut self, bit_count: u32, data: u32) -> Result<(), Self::Error> {
        (**self).write_bits(bit_count, data)
    }
    fn read_bits(&mut self, bit_count: u32) -> Result<u32, Self::Error> {
        (**self).read_bits(bit_count)
    }
    fn jtag_to_swd(&mut self) -> Result<(), Self::Error> {
        (**self).jtag_to_swd()
    }
    fn dormant_to_swd(&mut self) -> Result<(), Self::Error> {
        (**self).dormant_to_swd()
    }
    fn set_clock(&mut self, freq_khz: u32) -> Result<(), Self::Error> {
        (**self).set_clock(freq_khz)
    }
    fn set_turnaround(&mut self, cycles: u8) {
        (**self).set_turnaround(cycles);
    }
    fn set_idle_cycles(&mut self, cycles: u8) {
        (**self).set_idle_cycles(cycles);
    }
    fn assert_reset(&mut self, assert: bool) -> Result<(), Self::Error> {
        (**self).assert_reset(assert)
    }
    fn swj_pins(
        &mut self,
        pin_output: u8,
        pin_select: u8,
        wait_us: u32,
    ) -> Result<u8, Self::Error> {
        (**self).swj_pins(pin_output, pin_select, wait_us)
    }
}

pub trait SwioIo {
    type Error: Debug;

    /// Write 32-bit value to register address on WCH 1-wire SWIO bus.
    fn put(&mut self, addr: u32, data: u32) -> Result<(), Self::Error>;

    /// Read 32-bit value from register address on WCH 1-wire SWIO bus.
    fn get(&mut self, addr: u32) -> Result<u32, Self::Error>;

    /// Reset / awaken sequence on SWIO line.
    fn reset_sequence(&mut self) -> Result<(), Self::Error>;
}

impl<T: SwioIo + ?Sized> SwioIo for &mut T {
    type Error = T::Error;
    fn put(&mut self, addr: u32, data: u32) -> Result<(), Self::Error> {
        (**self).put(addr, data)
    }
    fn get(&mut self, addr: u32) -> Result<u32, Self::Error> {
        (**self).get(addr)
    }
    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        (**self).reset_sequence()
    }
}

pub trait RvswdIo {
    type Error: Debug;

    /// Write 32-bit value to register address on WCH 2-wire RVSWD bus.
    fn write(&mut self, reg: u8, value: u32) -> Result<(), Self::Error>;

    /// Read 32-bit value from register address on WCH 2-wire RVSWD bus.
    fn read(&mut self, reg: u8) -> Result<u32, Self::Error>;

    /// Reset sequence for RVSWD interface.
    fn reset(&mut self) -> Result<(), Self::Error>;
}

impl<T: RvswdIo + ?Sized> RvswdIo for &mut T {
    type Error = T::Error;
    fn write(&mut self, reg: u8, value: u32) -> Result<(), Self::Error> {
        (**self).write(reg, value)
    }
    fn read(&mut self, reg: u8) -> Result<u32, Self::Error> {
        (**self).read(reg)
    }
    fn reset(&mut self) -> Result<(), Self::Error> {
        (**self).reset()
    }
}
