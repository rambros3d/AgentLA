use crate::programmer::swd::{
    SwdTransport, ADIV5_AP_CSW_DEFAULT, AP_CSW, AP_DRW, AP_TAR,
};

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
}

pub struct CortexM<'a> {
    pub swd: &'a mut SwdTransport,
}

impl<'a> CortexM<'a> {
    pub fn new(swd: &'a mut SwdTransport) -> Self {
        Self { swd }
    }

    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, CortexError> {
        self.swd
            .write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap(AP_TAR, addr)
            .map_err(|_| CortexError::SwdError)?;
        let mut val = 0u32;
        self.swd
            .read_ap(AP_DRW, &mut val)
            .map_err(|_| CortexError::SwdError)?;
        Ok(val)
    }

    pub fn mem_write32(&mut self, addr: u32, value: u32) -> Result<(), CortexError> {
        self.swd
            .write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap(AP_TAR, addr)
            .map_err(|_| CortexError::SwdError)?;
        self.swd
            .write_ap(AP_DRW, value)
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
        self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT)?;
        for _ in 0..1000 {
            if let Ok(dhcsr) = self.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    return Ok(());
                }
            }
            cortex_m::asm::delay(1500); // 10 µs
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
            cortex_m::asm::delay(1500);
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
            cortex_m::asm::delay(1500);
        }
        Err(CortexError::Timeout)
    }

    pub fn connect_under_reset(&mut self) -> Result<u32, CortexError> {
        let reset_pin = self.swd.reset_pin;
        // 1. Assert nRST low
        crate::programmer::pins::drive_reset_pin(reset_pin, true);
        cortex_m::asm::delay(3_000_000); // 20 ms reset pulse

        // 2. Perform SWD line reset and switch to SWD
        self.swd.jtag_to_swd();
        let mut dpidr = 0u32;
        let _ = self.swd.read_dpidr_retry(&mut dpidr);
        let _ = self.swd.power_up_debug();

        // 3. Write DHCSR and DEMCR (arm vector catch) while reset is held low
        let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
        let _ = self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET);
        cortex_m::asm::delay(150_000); // 1 ms settling

        // 4. Release nRST
        crate::programmer::pins::drive_reset_pin(reset_pin, false);

        // 5. Poll DHCSR.S_HALT up to 500 ms (50_000 iterations x 10 µs)
        for i in 0..50_000 {
            if let Ok(dhcsr) = self.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    let pc = self.read_core_reg(15)?;
                    return Ok(pc);
                }
            }
            // After 50 ms (5000 iterations), if not halted, fallback to software SYSRESETREQ
            if i == 5000 {
                let _ = self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_HALT);
                let _ = self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET);
                let _ = self.mem_write32(AIRCR, AIRCR_VECTKEY | AIRCR_SYSRESETREQ);
            }
            cortex_m::asm::delay(1500); // 10 µs
        }
        Err(CortexError::ResetTimeout)
    }

    pub fn reset_and_catch(&mut self) -> Result<u32, CortexError> {
        // 1. Arm vector catch
        self.mem_write32(DHCSR, DHCSR_DBGKEY | DHCSR_C_DEBUGEN)?;
        self.mem_write32(DEMCR, DEMCR_TRCENA | DEMCR_VC_CORERESET)?;

        // 2. Trigger software system reset
        self.mem_write32(AIRCR, AIRCR_VECTKEY | AIRCR_SYSRESETREQ)?;

        // 3. Poll DHCSR.S_HALT up to 500 ms
        for _ in 0..50_000 {
            if let Ok(dhcsr) = self.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    let pc = self.read_core_reg(15)?;
                    return Ok(pc);
                }
            }
            cortex_m::asm::delay(1500); // 10 µs
        }
        Err(CortexError::ResetTimeout)
    }
}
