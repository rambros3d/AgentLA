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

    pub fn is_armv8m_dpidr(dpidr: u32) -> bool {
        dpidr == 0x6BA02477 || dpidr == 0x0BE12477
    }

    /// Configure Access Port and CSW based on DPIDR.
    /// For ARMv8-M / Cortex-M33 (e.g. STM32H5), routes to AP 1 (AHB-AP) with CSW = 0x23000012.
    /// For legacy ARMv7-M, defaults to AP 0 with standard CSW.
    pub fn auto_configure_for_dpidr(&mut self, dpidr: u32) -> Result<(), CortexError> {
        if Self::is_armv8m_dpidr(dpidr) {
            self.select_ap(1)?;
            self.csw = 0x23000012;
        } else {
            self.select_ap(0)?;
            self.csw = ADIV5_AP_CSW_DEFAULT;
        }
        Ok(())
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
        if self.swd.read_ap_port(self.apsel, AP_DRW, &mut val).is_err() {
            if self.apsel == 1 {
                self.swd
                    .write_ap_port(self.apsel, AP_CSW, 0xE3000012)
                    .map_err(|_| CortexError::SwdError)?;
                self.swd
                    .write_ap_port(self.apsel, AP_TAR, addr)
                    .map_err(|_| CortexError::SwdError)?;
                self.swd
                    .read_ap_port(self.apsel, AP_DRW, &mut val)
                    .map_err(|_| CortexError::SwdError)?;
                self.csw = 0xE3000012;
            } else {
                return Err(CortexError::SwdError);
            }
        }
        Ok(val)
    }

    pub fn mem_write32(&mut self, addr: u32, value: u32) -> Result<(), CortexError> {
        if self.swd.write_ap_port(self.apsel, AP_CSW, self.csw).is_err()
            || self.swd.write_ap_port(self.apsel, AP_TAR, addr).is_err()
            || self.swd.write_ap_port(self.apsel, AP_DRW, value).is_err()
        {
            if self.apsel == 1 {
                self.swd
                    .write_ap_port(self.apsel, AP_CSW, 0xE3000012)
                    .map_err(|_| CortexError::SwdError)?;
                self.swd
                    .write_ap_port(self.apsel, AP_TAR, addr)
                    .map_err(|_| CortexError::SwdError)?;
                self.swd
                    .write_ap_port(self.apsel, AP_DRW, value)
                    .map_err(|_| CortexError::SwdError)?;
                self.csw = 0xE3000012;
            } else {
                return Err(CortexError::SwdError);
            }
        }
        Ok(())
    }

    pub fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), CortexError> {
        let mut offset = 0usize;
        let mut curr_addr = addr;
        while offset < buf.len() {
            // TAR auto-increment is guaranteed only within 1 KiB
            let boundary_rem = (1024 - (curr_addr & 1023)) as usize;
            let chunk_len = boundary_rem.min(buf.len() - offset);

            let csw = if self.apsel == 1 { self.csw | 0xC0000000 } else { self.csw };
            self.swd
                .write_ap_port(self.apsel, AP_CSW, csw)
                .map_err(|_| CortexError::SwdError)?;

            let mut sub_offset = 0usize;
            while sub_offset < chunk_len {
                let sub_rem = chunk_len - sub_offset;
                let sub_words = (sub_rem / 4).min(64);
                let sub_addr = curr_addr + sub_offset as u32;

                if sub_words > 0 {
                    let mut words = [0u32; 64];
                    if self
                        .swd
                        .mem_read_chunk(self.apsel, sub_addr, &mut words[..sub_words])
                        .is_ok()
                    {
                        for i in 0..sub_words {
                            let bytes = words[i].to_le_bytes();
                            let target_idx = offset + sub_offset + i * 4;
                            buf[target_idx..target_idx + 4].copy_from_slice(&bytes);
                        }
                    } else {
                        for i in 0..sub_words {
                            let w = self.mem_read32(sub_addr + (i * 4) as u32)?;
                            let bytes = w.to_le_bytes();
                            let target_idx = offset + sub_offset + i * 4;
                            buf[target_idx..target_idx + 4].copy_from_slice(&bytes);
                        }
                    }
                    sub_offset += sub_words * 4;
                } else {
                    let mut single_word = [0u32; 1];
                    if self
                        .swd
                        .mem_read_chunk(self.apsel, sub_addr, &mut single_word)
                        .is_ok()
                    {
                        let bytes = single_word[0].to_le_bytes();
                        let target_idx = offset + sub_offset;
                        buf[target_idx..target_idx + sub_rem].copy_from_slice(&bytes[..sub_rem]);
                    } else {
                        let w = self.mem_read32(sub_addr)?;
                        let bytes = w.to_le_bytes();
                        let target_idx = offset + sub_offset;
                        buf[target_idx..target_idx + sub_rem].copy_from_slice(&bytes[..sub_rem]);
                    }
                    sub_offset += sub_rem;
                }
            }

            offset += chunk_len;
            curr_addr += chunk_len as u32;
        }
        Ok(())
    }

    pub fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), CortexError> {
        let mut offset = 0usize;
        let mut curr_addr = addr;
        while offset < data.len() {
            // TAR auto-increment is guaranteed only within 1 KiB
            let boundary_rem = (1024 - (curr_addr & 1023)) as usize;
            let chunk_len = boundary_rem.min(data.len() - offset);

            self.swd
                .write_ap_port(self.apsel, AP_CSW, self.csw)
                .map_err(|_| CortexError::SwdError)?;

            let mut sub_offset = 0usize;
            while sub_offset < chunk_len {
                let sub_rem = chunk_len - sub_offset;
                let sub_words = (sub_rem / 4).min(64);
                let sub_addr = curr_addr + sub_offset as u32;

                if sub_words > 0 {
                    let mut words = [0u32; 64];
                    for i in 0..sub_words {
                        let idx = offset + sub_offset + i * 4;
                        words[i] = u32::from_le_bytes([
                            data[idx],
                            data[idx + 1],
                            data[idx + 2],
                            data[idx + 3],
                        ]);
                    }
                    self.swd
                        .mem_write_chunk(self.apsel, sub_addr, &words[..sub_words])
                        .map_err(|_| CortexError::SwdError)?;
                    sub_offset += sub_words * 4;
                } else {
                    let mut chunk = [0xFFu8; 4];
                    let idx = offset + sub_offset;
                    chunk[..sub_rem].copy_from_slice(&data[idx..idx + sub_rem]);
                    let single_word = [u32::from_le_bytes(chunk)];
                    self.swd
                        .mem_write_chunk(self.apsel, sub_addr, &single_word)
                        .map_err(|_| CortexError::SwdError)?;
                    sub_offset += sub_rem;
                }
            }

            offset += chunk_len;
            curr_addr += chunk_len as u32;
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
        let _ = self.select_ap(self.apsel);

        if is_m33 {
            self.csw = 0xE3000012;
            let _ = self.mem_write32(0x4402_4004, 0x06); // DBG_STOP | DBG_STANDBY
            let _ = self.mem_write32(0x4402_4008, 0x1800); // DBG_WWDG_STOP | DBG_IWDG_STOP
            self.csw = 0x23000012;
        }

        // Release reset
        let _ = self.swd.assert_reset(false);

        // Immediate burst write across reset rising edge
        let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        let _ = self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET);
        for _ in 0..10 {
            let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        }

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
        // Hold reset low
        let _ = self.swd.assert_reset(true);
        delay_cycles(750_000); // 5 ms low

        let mut dpidr = 0u32;
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let is_m33 = dpidr == 0x6BA02477 || dpidr == 0x0BE12477;
        self.apsel = if is_m33 { 1 } else { 0 };
        self.csw = 0x23000012;
        let _ = self.swd.power_up_debug();
        let _ = self.select_ap(self.apsel);

        if is_m33 {
            self.csw = 0xE3000012;
            let _ = self.mem_write32(0x4402_4004, 0x06); // DBG_STOP | DBG_STANDBY
            let _ = self.mem_write32(0x4402_4008, 0x1800); // DBG_WWDG_STOP | DBG_IWDG_STOP
            self.csw = 0x23000012;
        }

        // Release hardware reset
        let _ = self.swd.assert_reset(false);

        // Immediate burst write across reset rising edge
        let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        let _ = self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET);
        for _ in 0..10 {
            let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        }

        // Re-attach and re-power debug
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
