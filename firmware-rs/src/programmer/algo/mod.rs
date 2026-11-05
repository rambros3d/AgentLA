pub mod blobs;

pub use blobs::{AlgoBlobHeader, STM32F1_ALGO_CODE, STM32F1_ALGO_HDR};
use crate::programmer::target::cortex_m::{CortexM, DHCSR, DHCSR_S_HALT};
use crate::programmer::target::targets::{TargetFamily, TargetInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoError {
    SwdError,
    Timeout,
    ExecutionFailed(u32),
    VerifyMismatch { addr: u32, expected: u8, actual: u8 },
    UnsupportedTarget,
}

pub struct FlashAlgorithmRunner;

impl FlashAlgorithmRunner {
    pub fn download_ram(cortex: &mut CortexM, addr: u32, data: &[u8]) -> Result<(), AlgoError> {
        cortex
            .mem_write_block(addr, data)
            .map_err(|_| AlgoError::SwdError)
    }

    pub fn exec_algo(
        cortex: &mut CortexM,
        entry: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
        sp: u32,
        timeout_ms: u32,
    ) -> Result<u32, AlgoError> {
        // 1. Halt core
        cortex.halt().map_err(|_| AlgoError::SwdError)?;

        // 2. Set up registers R0-R3, SP, LR, PC
        cortex
            .write_core_reg(0, r0)
            .map_err(|_| AlgoError::SwdError)?;
        cortex
            .write_core_reg(1, r1)
            .map_err(|_| AlgoError::SwdError)?;
        cortex
            .write_core_reg(2, r2)
            .map_err(|_| AlgoError::SwdError)?;
        cortex
            .write_core_reg(3, r3)
            .map_err(|_| AlgoError::SwdError)?;

        if sp != 0 {
            cortex
                .write_core_reg(13, sp)
                .map_err(|_| AlgoError::SwdError)?; // SP
        }

        // LR = 0 (or breakpoint return address)
        cortex
            .write_core_reg(14, 0x00000000)
            .map_err(|_| AlgoError::SwdError)?;

        // PC = entry | 1 (Thumb bit)
        cortex
            .write_core_reg(15, entry | 1)
            .map_err(|_| AlgoError::SwdError)?;

        // 3. Resume core
        cortex.resume().map_err(|_| AlgoError::SwdError)?;

        // 4. Synchronous execution monitor: poll DHCSR for HALT / BKPT
        let max_iterations = timeout_ms * 100;
        for _ in 0..max_iterations {
            if let Ok(dhcsr) = cortex.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    let result = cortex
                        .read_core_reg(0)
                        .map_err(|_| AlgoError::SwdError)?;
                    return Ok(result);
                }
            }
            cortex_m::asm::delay(1500); // 10 µs
        }

        // Timeout: halt core
        let _ = cortex.halt();
        Err(AlgoError::Timeout)
    }

    pub fn flm_erase_sector(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        sector_addr: u32,
    ) -> Result<(), AlgoError> {
        let entry = code_base + hdr.erase_offset;
        let res = Self::exec_algo(
            cortex,
            entry,
            sector_addr,
            0,
            0,
            0,
            hdr.stack_top,
            2000,
        )?;
        if res != 0 {
            return Err(AlgoError::ExecutionFailed(res));
        }
        Ok(())
    }

    pub fn flm_program_page(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        page_addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        // 1. Download payload data into SRAM buffer
        Self::download_ram(cortex, hdr.ram_base, data)?;

        // 2. Invoke ProgramPage(page_addr, size, ram_base)
        let entry = code_base + hdr.program_offset;
        let res = Self::exec_algo(
            cortex,
            entry,
            page_addr,
            data.len() as u32,
            hdr.ram_base,
            0,
            hdr.stack_top,
            2000,
        )?;
        if res != 0 {
            return Err(AlgoError::ExecutionFailed(res));
        }
        Ok(())
    }

    pub fn flm_verify(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        addr: u32,
        data: &[u8],
    ) -> Result<bool, AlgoError> {
        if hdr.verify_offset == 0 {
            // Direct readback verify
            let mut buf = [0u8; 64];
            let mut offset = 0usize;
            while offset < data.len() {
                let chunk_len = (data.len() - offset).min(buf.len());
                cortex
                    .mem_read_block(addr + offset as u32, &mut buf[..chunk_len])
                    .map_err(|_| AlgoError::SwdError)?;
                if buf[..chunk_len] != data[offset..offset + chunk_len] {
                    return Ok(false);
                }
                offset += chunk_len;
            }
            return Ok(true);
        }

        Self::download_ram(cortex, hdr.ram_base, data)?;
        let entry = code_base + hdr.verify_offset;
        let res = Self::exec_algo(
            cortex,
            entry,
            addr,
            data.len() as u32,
            hdr.ram_base,
            0,
            hdr.stack_top,
            2000,
        )?;
        Ok(res == 0)
    }

    // Direct hardware flash unlock and operations for STM32F1 & PY32
    pub fn stm32f1_unlock(cortex: &mut CortexM) -> Result<(), AlgoError> {
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
        let mut cr = 0u32;
        if cortex.swd.read_ap(crate::programmer::swd::AP_DRW, &mut cr).is_ok() && (cr & 0x80) == 0 {
            // Already unlocked
            return Ok(());
        }
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022004);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x45670123);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022004);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0xCDEF89AB);
        // Clear status register error flags (PGERR, WRPRTERR, EOP)
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x4002200C);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x34);
        Ok(())
    }

    pub fn stm32f1_wait_bsy(cortex: &mut CortexM) -> Result<(), AlgoError> {
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
        for _ in 0..10_000 {
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x4002200C);
            let mut sr = 0u32;
            if cortex.swd.read_ap(crate::programmer::swd::AP_DRW, &mut sr).is_ok() {
                if (sr & 0x01) == 0 {
                    if (sr & 0x14) != 0 {
                        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x4002200C);
                        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x34);
                        return Err(AlgoError::ExecutionFailed(sr));
                    }
                    return Ok(());
                }
            }
            cortex_m::asm::delay(1500);
        }
        Err(AlgoError::Timeout)
    }

    pub fn stm32f1_erase_sector(cortex: &mut CortexM, sector_addr: u32) -> Result<(), AlgoError> {
        Self::stm32f1_unlock(cortex)?;
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x4002200C);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x34); // Clear EOP and error flags

        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x02); // PER=1

        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022014);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, sector_addr); // AR = sector_addr

        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x42); // STRT=1, PER=1

        Self::stm32f1_wait_bsy(cortex)?;

        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x00); // Clear PER
        Ok(())
    }

    pub fn stm32f1_program_page(
        cortex: &mut CortexM,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        Self::stm32f1_unlock(cortex)?;
        let mut offset = 0usize;
        while offset < data.len() {
            let halfword = if offset + 1 < data.len() {
                (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
            } else {
                data[offset] as u16 | 0xFF00
            };

            // 1. Clear EOP and error flags in FLASH_SR
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x4002200C);
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x34);

            // 2. Set PG=1 in FLASH_CR
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x01);

            // 3. Program halfword using 16-bit CSW access with byte lane positioning
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_16BIT);
            let curr_addr = addr + offset as u32;
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, curr_addr);
            let packed = (halfword as u32) << (8 * (curr_addr & 2));
            let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, packed);

            // 4. Wait for BSY clear
            Self::stm32f1_wait_bsy(cortex)?;
            offset += 2;
        }

        // Clear PG bit
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x00);
        Ok(())
    }

    // High level flash write + verify for any target
    pub fn flash_target(
        cortex: &mut CortexM,
        target: &TargetInfo,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        match target.family {
            TargetFamily::Stm32F1 | TargetFamily::Py32F0 => {
                // Erase sectors
                let sector_size = target.sector_size;
                let mut curr_sector = addr & !(sector_size - 1);
                let end_addr = addr + data.len() as u32;
                while curr_sector < end_addr {
                    Self::stm32f1_erase_sector(cortex, curr_sector)?;
                    curr_sector += sector_size;
                }
                // Program pages
                let mut offset = 0usize;
                while offset < data.len() {
                    let chunk_len = (data.len() - offset).min(target.page_size as usize);
                    Self::stm32f1_program_page(
                        cortex,
                        addr + offset as u32,
                        &data[offset..offset + chunk_len],
                    )?;
                    offset += chunk_len;
                }
                // Verify
                let mut readback = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk_len = (data.len() - v_offset).min(readback.len());
                    cortex
                        .mem_read_block(addr + v_offset as u32, &mut readback[..chunk_len])
                        .map_err(|_| AlgoError::SwdError)?;
                    for i in 0..chunk_len {
                        if readback[i] != data[v_offset + i] {
                            return Err(AlgoError::VerifyMismatch {
                                addr: addr + (v_offset + i) as u32,
                                expected: data[v_offset + i],
                                actual: readback[i],
                            });
                        }
                    }
                    v_offset += chunk_len;
                }
                Ok(())
            }
            _ => {
                // For other Cortex targets, download and run FLM blob runner
                let code_base = target.ram_base;
                Self::download_ram(cortex, code_base, STM32F1_ALGO_CODE)?;
                let hdr = STM32F1_ALGO_HDR;
                Self::flm_erase_sector(cortex, &hdr, code_base, addr)?;
                Self::flm_program_page(cortex, &hdr, code_base, addr, data)?;
                let ok = Self::flm_verify(cortex, &hdr, code_base, addr, data)?;
                if !ok {
                    return Err(AlgoError::ExecutionFailed(1));
                }
                Ok(())
            }
        }
    }
}
