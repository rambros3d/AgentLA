//! Arm Cortex-M core debug operations over ADIv5.

use crate::delay::delay_cycles;
use crate::swd::{Adiv5Access, ADIV5_AP_CSW_DEFAULT, AP_CSW, AP_DRW, AP_TAR};

pub const DHCSR: u32 = 0xE000EDF0;
pub const DCRSR: u32 = 0xE000EDF4;
pub const DCRDR: u32 = 0xE000EDF8;
pub const DEMCR: u32 = 0xE000EDFC;
pub const AIRCR: u32 = 0xE000ED0C;

pub const DHCSR_DBGKEY: u32 = 0xA05F0000;
pub const DHCSR_C_DEBUGEN: u32 = 0x00000001;
pub const DHCSR_C_HALT: u32 = 0x00000002;
pub const DHCSR_C_STEP: u32 = 0x00000004;
pub const DHCSR_S_REGRDY: u32 = 0x00010000;
pub const DHCSR_S_HALT: u32 = 0x00020000;

pub const DEMCR_TRCENA: u32 = 1 << 24;
pub const DEMCR_VC_CORERESET: u32 = 1 << 0;

pub const AIRCR_VECTKEY: u32 = 0x05FA0000;
pub const AIRCR_SYSRESETREQ: u32 = 1 << 2;
pub const AIRCR_VECTRESET: u32 = 1 << 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexError {
    SwdError,
    Timeout,
    HaltFailed,
    NotHalted,
    ResetTimeout,
    Diag(u8, u32, u32),
}

pub struct CortexM<'a, S: Adiv5Access> {
    pub swd: &'a mut S,
    pub apsel: u8,
    pub csw: u32,
}

impl<'a, S: Adiv5Access> CortexM<'a, S> {
    pub fn new(swd: &'a mut S) -> Self {
        Self {
            swd,
            apsel: 0,
            csw: ADIV5_AP_CSW_DEFAULT,
        }
    }

    pub fn with_csw(mut self, csw: u32) -> Self {
        self.csw = csw;
        self
    }

    pub fn with_apsel(mut self, apsel: u8) -> Self {
        self.apsel = apsel;
        self
    }

    pub fn set_csw(&mut self, csw: u32) {
        self.csw = csw;
    }

    pub fn select_ap(&mut self, apsel: u8) -> Result<(), CortexError> {
        self.apsel = apsel;
        self.swd
            .select_ap(apsel, 0)
            .map_err(|_| CortexError::SwdError)
    }

    pub fn read_ap_reg(&mut self, apsel: u8, reg: u8) -> Result<u32, CortexError> {
        let mut val = 0u32;
        self.swd
            .read_ap_port(apsel, reg, &mut val)
            .map_err(|_| CortexError::SwdError)?;
        Ok(val)
    }

    pub fn write_ap_reg(&mut self, apsel: u8, reg: u8, val: u32) -> Result<(), CortexError> {
        self.swd
            .write_ap_port(apsel, reg, val)
            .map_err(|_| CortexError::SwdError)?;
        Ok(())
    }

    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, CortexError> {
        self.swd
            .write_ap_port(self.apsel, AP_CSW, self.csw)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap_port(self.apsel, AP_TAR, addr)
            .map_err(|_| CortexError::SwdError)?;
        let mut val = 0u32;
        self.swd
            .read_ap_port(self.apsel, AP_DRW, &mut val)
            .map_err(|_| CortexError::SwdError)?;
        Ok(val)
    }

    pub fn mem_write32(&mut self, addr: u32, value: u32) -> Result<(), CortexError> {
        self.swd
            .write_ap_port(self.apsel, AP_CSW, self.csw)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap_port(self.apsel, AP_TAR, addr)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap_port(self.apsel, AP_DRW, value)
            .map_err(|_| CortexError::SwdError)?;
        Ok(())
    }

    pub fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), CortexError> {
        let mut offset = 0usize;
        let mut curr_addr = addr;
        while offset + 4 <= buf.len() {
            let val = self.mem_read32(curr_addr)?;
            buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
            offset += 4;
            curr_addr += 4;
        }
        if offset < buf.len() {
            let val = self.mem_read32(curr_addr)?;
            let bytes = val.to_le_bytes();
            let rem = buf.len() - offset;
            buf[offset..offset + rem].copy_from_slice(&bytes[..rem]);
        }
        Ok(())
    }

    pub fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), CortexError> {
        let mut offset = 0usize;
        let mut curr_addr = addr;
        while offset + 4 <= data.len() {
            let val = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            self.mem_write32(curr_addr, val)?;
            offset += 4;
            curr_addr += 4;
        }
        if offset < data.len() {
            let mut chunk = [0xFFu8; 4];
            let rem = data.len() - offset;
            chunk[..rem].copy_from_slice(&data[offset..]);
            let val = u32::from_le_bytes(chunk);
            self.mem_write32(curr_addr, val)?;
        }
        Ok(())
    }

    pub fn halt(&mut self) -> Result<(), CortexError> {
        // Step 1: Enable debug if not already enabled (ARMv8-M requirement)
        let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN);
        // Step 2: Request halt with debug enabled
        self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT)?;
        for _ in 0..1000 {
            if let Ok(dhcsr) = self.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    return Ok(());
                }
            }
            delay_cycles(1500); // 10 µs
        }
        Err(CortexError::HaltFailed)
    }

    pub fn resume(&mut self) -> Result<(), CortexError> {
        self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN)?;
        Ok(())
    }

    pub fn is_halted(&mut self) -> Result<bool, CortexError> {
        let dhcsr = self.mem_read32(DHCSR)?;
        Ok((dhcsr & DHCSR_S_HALT) != 0)
    }

    pub fn read_core_reg(&mut self, reg: u8) -> Result<u32, CortexError> {
        self.mem_write32(DCRSR, reg as u32 & 0x1F)?;
        for _ in 0..1000 {
            let dhcsr = self.mem_read32(DHCSR)?;
            if (dhcsr & DHCSR_S_REGRDY) != 0 {
                return self.mem_read32(DCRDR);
            }
            delay_cycles(1500);
        }
        Err(CortexError::Timeout)
    }

    pub fn write_core_reg(&mut self, reg: u8, value: u32) -> Result<(), CortexError> {
        self.mem_write32(DCRDR, value)?;
        self.mem_write32(DCRSR, (1 << 16) | (reg as u32 & 0x1F))?;
        for _ in 0..1000 {
            let dhcsr = self.mem_read32(DHCSR)?;
            if (dhcsr & DHCSR_S_REGRDY) != 0 {
                return Ok(());
            }
            delay_cycles(1500);
        }
        Err(CortexError::Timeout)
    }

    pub fn connect_under_reset(&mut self) -> Result<u32, CortexError> {
        // 1. Try reset_and_catch first (clean hardware vector catch + reset pulse)
        if let Ok(pc) = self.reset_and_catch() {
            return Ok(pc);
        }

        // 2. Hardware connect under reset:
        // Hold reset low
        let _ = self.swd.assert_reset(true);
        delay_cycles(750_000); // 5 ms low

        // Attach while reset is low
        let mut dpidr = 0u32;
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let is_m33 = dpidr == 0x6BA02477 || dpidr == 0x0BE12477;
        self.apsel = if is_m33 { 1 } else { 0 };
        self.csw = 0x23000012;
        let _ = self.swd.power_up_debug();

        // Release reset
        let _ = self.swd.assert_reset(false);
        delay_cycles(150_000); // 1 ms settling

        // Re-establish SWD sync and debug power after reset release
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let _ = self.swd.power_up_debug();
        let _ = self.select_ap(self.apsel);

        // Request halt
        let _ = self.halt();
        if let Ok(pc) = self.read_core_reg(15) {
            return Ok(pc);
        }

        // Check if halted anyway
        if let Ok(dhcsr) = self.mem_read32(DHCSR) {
            if (dhcsr & DHCSR_S_HALT) != 0 {
                let pc = self.read_core_reg(15).unwrap_or(0);
                return Ok(pc);
            }
            return Err(CortexError::Diag(3, dpidr, dhcsr));
        }

        Err(CortexError::Diag(1, dpidr, 0))
    }

    pub fn reset_and_catch(&mut self) -> Result<u32, CortexError> {
        let mut dpidr = 0u32;
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let is_m33 = dpidr == 0x6BA02477 || dpidr == 0x0BE12477;
        self.apsel = if is_m33 { 1 } else { 0 };
        self.csw = 0x23000012;
        let _ = self.swd.power_up_debug();
        let _ = self.select_ap(self.apsel);

        // 1. Arm vector catch and enable halting debug
        let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN);
        let _ = self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET);

        // 2. Pulse hardware reset
        let _ = self.swd.assert_reset(true);
        delay_cycles(750_000); // 5 ms low
        let _ = self.swd.assert_reset(false);
        delay_cycles(150_000); // 1 ms settling

        // 3. Re-attach and re-power debug
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let _ = self.swd.power_up_debug();
        let _ = self.select_ap(self.apsel);

        // 4. Poll DHCSR for halt
        for i in 0..1000 {
            if let Ok(dhcsr) = self.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    let pc = self.read_core_reg(15).unwrap_or(0);
                    return Ok(pc);
                }
            }
            if i < 20 || i % 20 == 0 {
                let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
            }
            delay_cycles(1500); // 10 µs
        }

        // 5. Explicit halt fallback
        let _ = self.halt();
        if let Ok(pc) = self.read_core_reg(15) {
            return Ok(pc);
        }

        Err(CortexError::ResetTimeout)
    }
}

impl<'a, S: Adiv5Access> agentla_targets::TargetMemAccess for CortexM<'a, S> {
    type Error = CortexError;

    fn read32(&mut self, addr: u32) -> Result<u32, Self::Error> {
        self.mem_read32(addr)
    }

    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error> {
        self.mem_write32(addr, val)
    }

    fn read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.mem_read_block(addr, buf)
    }

    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error> {
        self.mem_write_block(addr, data)
    }

    fn assert_reset(&mut self, assert: bool) -> Result<(), Self::Error> {
        self.swd.assert_reset(assert).map_err(|_| CortexError::SwdError)
    }

    fn halt(&mut self) -> Result<(), Self::Error> {
        CortexM::halt(self)
    }

    fn resume(&mut self) -> Result<(), Self::Error> {
        CortexM::resume(self)
    }
}
