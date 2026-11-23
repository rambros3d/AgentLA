//! CMSIS FLM flash algorithm execution engine and direct flash controllers.

pub mod blobs;

pub use blobs::{AlgoBlobHeader, ALGO_MAGIC, ALGO_VERSION, STM32F1_ALGO_CODE, STM32F1_ALGO_HDR};
use crate::delay::delay_cycles;
use crate::swd::{Adiv5Access, ADIV5_AP_CSW_16BIT, ADIV5_AP_CSW_DEFAULT, AP_CSW, AP_DRW, AP_TAR};
use crate::target::cortex_m::{CortexM, DHCSR, DHCSR_S_HALT};
use crate::target::targets::{TargetFamily, TargetInfo};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlgoError {
    SwdError,
    Timeout,
    ExecutionFailed(u32),
    VerifyMismatch { addr: u32, expected: u8, actual: u8 },
    UnsupportedTarget,
    AlgoNotFound,
    AlgoInvalid,
}

pub struct FlashAlgorithmRunner;

impl FlashAlgorithmRunner {
    pub fn download_ram<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        cortex
            .mem_write_block(addr, data)
            .map_err(|_| AlgoError::SwdError)
    }

    pub fn start_algo<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        entry: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
        sp: u32,
    ) -> Result<(), AlgoError> {
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

        // LR points to BKPT 0 opcode (0xBE00) placed at sp - 4
        let lr = if sp >= 4 {
            let bkpt_addr = sp - 4;
            let _ = cortex.mem_write32(bkpt_addr, 0xBE00BE00);
            bkpt_addr | 1
        } else {
            0x00000000
        };
        cortex
            .write_core_reg(14, lr)
            .map_err(|_| AlgoError::SwdError)?;

        // PC = entry | 1 (Thumb bit)
        cortex
            .write_core_reg(15, entry | 1)
            .map_err(|_| AlgoError::SwdError)?;

        // 3. Resume core non-blocking
        cortex.resume().map_err(|_| AlgoError::SwdError)?;
        Ok(())
    }

    pub fn wait_algo_halt<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        timeout_ms: u32,
    ) -> Result<u32, AlgoError> {
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
            delay_cycles(1500); // 10 µs
        }

        let _ = cortex.halt();
        Err(AlgoError::Timeout)
    }

    pub fn exec_algo<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        entry: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
        sp: u32,
        timeout_ms: u32,
    ) -> Result<u32, AlgoError> {
        Self::start_algo(cortex, entry, r0, r1, r2, r3, sp)?;
        Self::wait_algo_halt(cortex, timeout_ms)
    }

    pub fn flm_erase_sector<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
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

    pub fn flm_program_page<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        page_addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        Self::download_ram(cortex, hdr.ram_base, data)?;
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

    pub fn flm_verify<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        addr: u32,
        data: &[u8],
    ) -> Result<bool, AlgoError> {
        if hdr.verify_offset == 0 {
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

    pub fn stm32f1_unlock<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), AlgoError> {
        let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(AP_TAR, 0x40022004);
        let _ = cortex.swd.write_ap(AP_DRW, 0x45670123);
        let _ = cortex.swd.write_ap(AP_TAR, 0x40022004);
        let _ = cortex.swd.write_ap(AP_DRW, 0xCDEF89AB);
        let _ = cortex.swd.write_ap(AP_TAR, 0x4002200C);
        let _ = cortex.swd.write_ap(AP_DRW, 0x34);
        Ok(())
    }

    pub fn stm32f1_wait_bsy<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), AlgoError> {
        let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        for _ in 0..10_000 {
            let _ = cortex.swd.write_ap(AP_TAR, 0x4002200C);
            let mut sr = 0u32;
            if cortex.swd.read_ap(AP_DRW, &mut sr).is_ok() {
                if (sr & 0x01) == 0 {
                    if (sr & 0x14) != 0 {
                        let _ = cortex.swd.write_ap(AP_TAR, 0x4002200C);
                        let _ = cortex.swd.write_ap(AP_DRW, 0x34);
                        return Err(AlgoError::ExecutionFailed(sr));
                    }
                    return Ok(());
                }
            }
            delay_cycles(1500);
        }
        Err(AlgoError::Timeout)
    }

    pub fn stm32f1_erase_sector<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        sector_addr: u32,
    ) -> Result<(), AlgoError> {
        Self::stm32f1_unlock(cortex)?;
        let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(AP_TAR, 0x4002200C);
        let _ = cortex.swd.write_ap(AP_DRW, 0x34);

        let _ = cortex.swd.write_ap(AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(AP_DRW, 0x02);

        let _ = cortex.swd.write_ap(AP_TAR, 0x40022014);
        let _ = cortex.swd.write_ap(AP_DRW, sector_addr);

        let _ = cortex.swd.write_ap(AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(AP_DRW, 0x42);

        Self::stm32f1_wait_bsy(cortex)?;

        let _ = cortex.swd.write_ap(AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(AP_DRW, 0x00);
        Ok(())
    }

    pub fn stm32f1_program_page<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
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

            let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
            let _ = cortex.swd.write_ap(AP_TAR, 0x4002200C);
            let _ = cortex.swd.write_ap(AP_DRW, 0x34);

            let _ = cortex.swd.write_ap(AP_TAR, 0x40022010);
            let _ = cortex.swd.write_ap(AP_DRW, 0x01);

            let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_16BIT);
            let curr_addr = addr + offset as u32;
            let _ = cortex.swd.write_ap(AP_TAR, curr_addr);
            let packed = (halfword as u32) << (8 * (curr_addr & 2));
            let _ = cortex.swd.write_ap(AP_DRW, packed);

            Self::stm32f1_wait_bsy(cortex)?;
            offset += 2;
        }

        let _ = cortex.swd.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
        let _ = cortex.swd.write_ap(AP_TAR, 0x40022010);
        let _ = cortex.swd.write_ap(AP_DRW, 0x00);
        Ok(())
    }

pub const STM32H5_ALGO: &[u8] = &[
    0x01, 0x28, 0x5b, 0xd0, 0x02, 0x28, 0x40, 0xf0, 0x8f, 0x80, 0x49, 0x48,
    0x00, 0xb5, 0xd0, 0xf8, 0x28, 0xc0, 0x1c, 0xf0, 0x01, 0x0f, 0x3d, 0xd1,
    0x4f, 0xf4, 0x7f, 0x00, 0xdf, 0xf8, 0x10, 0xc1, 0xcc, 0xf8, 0x30, 0x00,
    0xdc, 0xf8, 0x20, 0x00, 0x10, 0xf0, 0x0b, 0x0f, 0xfa, 0xd1, 0xdc, 0xf8,
    0x28, 0x00, 0x1b, 0x09, 0x40, 0xf0, 0x02, 0x00, 0xcc, 0xf8, 0x28, 0x00,
    0x1f, 0xd0, 0x3c, 0x48, 0x94, 0x46, 0x02, 0xeb, 0x03, 0x1e, 0x8a, 0x1a,
    0xdc, 0xf8, 0x00, 0x10, 0x02, 0xeb, 0x0c, 0x03, 0x42, 0xf8, 0x0c, 0x10,
    0xdc, 0xf8, 0x04, 0x10, 0x59, 0x60, 0xdc, 0xf8, 0x08, 0x10, 0x99, 0x60,
    0xdc, 0xf8, 0x0c, 0x10, 0xd9, 0x60, 0x03, 0x6a, 0x13, 0xf0, 0x0b, 0x0f,
    0xfb, 0xd1, 0x03, 0x6a, 0x13, 0xf4, 0x7e, 0x0f, 0x17, 0xd1, 0x0c, 0xf1,
    0x10, 0x0c, 0xe6, 0x45, 0xe4, 0xd1, 0x4f, 0xf4, 0x7f, 0x01, 0x2b, 0x4b,
    0x00, 0x20, 0x9a, 0x6a, 0x22, 0xf0, 0x02, 0x02, 0x9a, 0x62, 0x19, 0x63,
    0x00, 0xbe, 0x00, 0xbd, 0xdf, 0xf8, 0x9c, 0xc0, 0xc0, 0xf8, 0x04, 0xc0,
    0x0c, 0xf1, 0x88, 0x3c, 0xc0, 0xf8, 0x04, 0xc0, 0xb8, 0xe7, 0x4f, 0xf4,
    0x7f, 0x02, 0x83, 0x6a, 0x23, 0xf0, 0x02, 0x03, 0x83, 0x62, 0x02, 0x63,
    0x4f, 0xf0, 0xff, 0x30, 0x00, 0xbe, 0x00, 0xbd, 0x1c, 0x4b, 0x9a, 0x6a,
    0xd2, 0x07, 0x2c, 0xd4, 0x4f, 0xf4, 0x7f, 0x03, 0x19, 0x4a, 0x13, 0x63,
    0x13, 0x6a, 0x13, 0xf0, 0x0b, 0x0f, 0xfb, 0xd1, 0x18, 0x4b, 0x99, 0x42,
    0x1b, 0xd8, 0x24, 0x23, 0x01, 0xf1, 0x78, 0x41, 0x49, 0x0b, 0x13, 0x4a,
    0x43, 0xea, 0x81, 0x13, 0x93, 0x62, 0x13, 0x6a, 0x13, 0xf0, 0x0b, 0x0f,
    0xfb, 0xd1, 0x4f, 0xf4, 0x7f, 0x01, 0x90, 0x6a, 0x10, 0x4b, 0x03, 0x40,
    0x93, 0x62, 0x11, 0x63, 0x13, 0x6a, 0x13, 0xf4, 0x7e, 0x0f, 0x14, 0xbf,
    0x4f, 0xf0, 0xff, 0x30, 0x00, 0x20, 0x00, 0xbe, 0x70, 0x47, 0x49, 0x0b,
    0xa1, 0xf5, 0x80, 0x41, 0x09, 0x4b, 0x08, 0x39, 0xe1, 0xe7, 0x05, 0x48,
    0x08, 0x4a, 0x58, 0x60, 0x5a, 0x60, 0xcd, 0xe7, 0x4f, 0xf0, 0xff, 0x30,
    0xef, 0xe7, 0x00, 0xbf, 0x00, 0x20, 0x02, 0x50, 0x23, 0x01, 0x67, 0x45,
    0xff, 0xff, 0x00, 0x08, 0x3b, 0xe0, 0xff, 0x7f, 0x24, 0x00, 0x00, 0x80,
    0xab, 0x89, 0xef, 0xcd,
];

    pub fn stm32h5_run_algo<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        cmd: u32,
        arg1: u32,
        arg2: u32,
        arg3: u32,
    ) -> Result<(), AlgoError> {
        let _ = cortex.halt();
        let _ = cortex.swd.write_dp(crate::swd::DP_ABORT, 0x0000001E);
        let _ = cortex.select_ap(1);
        cortex.csw = 0x0B000012;
        let _ = cortex.swd.write_ap_port(1, crate::swd::AP_CSW, 0x0B000012);

        // Download algo to SRAM base 0x20000000
        cortex
            .mem_write_block(0x20000000, Self::STM32H5_ALGO)
            .map_err(|_| AlgoError::ExecutionFailed(0x8001))?;

        // Enable vector catch for hard fault and trace
        let _ = cortex.mem_write32(0xE000EDFC, 0x01000401);

        // Setup CPU registers
        cortex.write_core_reg(0, cmd).map_err(|_| AlgoError::ExecutionFailed(0x6003))?;
        cortex.write_core_reg(1, arg1).map_err(|_| AlgoError::ExecutionFailed(0x6004))?;
        cortex.write_core_reg(2, arg2).map_err(|_| AlgoError::ExecutionFailed(0x6005))?;
        cortex.write_core_reg(3, arg3).map_err(|_| AlgoError::ExecutionFailed(0x6006))?;

        // SP: 0x20002000 (8 KB into RAM)
        cortex.write_core_reg(13, 0x20002000).map_err(|_| AlgoError::ExecutionFailed(0x6007))?;

        // LR: 0x20001FFD (with bkpt opcode at 0x20001FFC)
        let _ = cortex.mem_write32(0x20001FFC, 0xBE00BE00);
        cortex.write_core_reg(14, 0x20001FFD).map_err(|_| AlgoError::ExecutionFailed(0x6008))?;

        // xPSR: Thumb state (bit 24 = 1)
        cortex.write_core_reg(16, 0x01000000).map_err(|_| AlgoError::ExecutionFailed(0x6009))?;

        // PC: entry point 0x20000000 (even address)
        cortex.write_core_reg(15, 0x20000000).map_err(|_| AlgoError::ExecutionFailed(0x600A))?;

        // Resume core execution
        cortex.resume().map_err(|_| AlgoError::ExecutionFailed(0x600B))?;

        // Wait for halt (bkpt #0 triggered)
        let max_iters = if cmd == 1 { 20_000 } else { 5_000 };
        for _ in 0..max_iters {
            if let Ok(dhcsr) = cortex.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    let r0 = cortex.read_core_reg(0).map_err(|_| AlgoError::ExecutionFailed(0x600C))?;
                    if r0 != 0 {
                        let nssr = cortex.mem_read32(0x40022020).unwrap_or(0);
                        return Err(AlgoError::ExecutionFailed(0x50000000 | (nssr & 0x00FFFFFF)));
                    }
                    return Ok(());
                }
            }
            delay_cycles(1500); // 10 µs
        }

        let _ = cortex.halt();
        let pc = cortex.read_core_reg(15).unwrap_or(0);
        Err(AlgoError::ExecutionFailed(0x5E000000 | (pc & 0xFFFF)))
    }

    pub fn stm32h5_erase_sector<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        sector_addr: u32,
    ) -> Result<(), AlgoError> {
        let _ = cortex.halt();
        Self::stm32h5_unlock(cortex)?;

        // Clear error flags: NSCCR (0x40022030) = 0x003F0000
        let _ = cortex.mem_write32(0x40022030, 0x003F0000);

        // Wait for BSY == 0 in NSSR (0x40022020)
        for _ in 0..10_000 {
            if (cortex.mem_read32(0x40022020).unwrap_or(0) & 0x01) == 0 {
                break;
            }
        }

        // Calculate sector number: (sector_addr - 0x08000000) / 8192
        let snb = sector_addr.saturating_sub(0x08000000) / 8192;
        // FLASH_CR: SER (bit 2) | STRT (bit 5) | (snb << 6)
        let cr_val = (1 << 2) | (1 << 5) | ((snb & 0x7F) << 6);
        let _ = cortex.mem_write32(0x40022028, cr_val);

        // Wait for erase completion (BSY == 0)
        let mut busy = true;
        for _ in 0..50_000 {
            let sr = cortex.mem_read32(0x40022020).unwrap_or(0);
            if (sr & 0x01) == 0 {
                busy = false;
                if (sr & 0x00FE0000) != 0 {
                    let _ = cortex.mem_write32(0x40022028, 0);
                    return Err(AlgoError::ExecutionFailed(0x50000000 | (sr & 0x00FFFFFF)));
                }
                break;
            }
            delay_cycles(1500);
        }
        let _ = cortex.mem_write32(0x40022028, 0);

        if busy {
            return Err(AlgoError::Timeout);
        }
        Ok(())
    }

    pub fn stm32h5_program_page<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        flash_addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        let _ = cortex.halt();
        Self::stm32h5_unlock(cortex)?;

        // Clear error flags: NSCCR (0x40022030)
        let _ = cortex.mem_write32(0x40022030, 0x00FE0000 | 0x00010000);

        // Set PG bit in NSCR (0x40022028)
        let _ = cortex.mem_write32(0x40022028, 0x02);

        // Ensure AHB-AP auto-increment single is configured
        cortex.csw = 0x0B000012;

        let mut offset = 0usize;
        while offset < data.len() {
            let chunk_size = (data.len() - offset).min(16);
            let mut qw = [0xFFu8; 16];
            qw[..chunk_size].copy_from_slice(&data[offset..offset + chunk_size]);
            let curr_addr = flash_addr + offset as u32;

            // Write 16-byte (128-bit) quad-word block
            cortex
                .mem_write_block(curr_addr, &qw)
                .map_err(|_| AlgoError::SwdError)?;

            // Wait for BSY == 0 in NSSR (0x40022020)
            let mut busy = true;
            for _ in 0..10_000 {
                let sr = cortex.mem_read32(0x40022020).unwrap_or(0);
                if (sr & 0x01) == 0 {
                    busy = false;
                    if (sr & 0x00FE0000) != 0 {
                        let _ = cortex.mem_write32(0x40022028, 0);
                        return Err(AlgoError::ExecutionFailed(0x50000000 | (sr & 0x00FFFFFF)));
                    }
                    break;
                }
                delay_cycles(150);
            }
            if busy {
                let _ = cortex.mem_write32(0x40022028, 0);
                return Err(AlgoError::Timeout);
            }

            offset += 16;
        }

        // Clear PG
        let _ = cortex.mem_write32(0x40022028, 0);
        Ok(())
    }

    pub fn stm32h5_unlock<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), AlgoError> {
        let cr = cortex.mem_read32(0x40022028).unwrap_or(1);
        if (cr & 1) != 0 {
            let _ = cortex.mem_write32(0x40022004, 0x45670123);
            let _ = cortex.mem_write32(0x40022004, 0xCDEF89AB);
        }
        Ok(())
    }

    pub fn stm32h5_wait_bsy<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), AlgoError> {
        for _ in 0..20_000 {
            let sr = cortex.mem_read32(0x40022020).unwrap_or(1);
            if (sr & 0x01) == 0 {
                return Ok(());
            }
            delay_cycles(150);
        }
        Err(AlgoError::Timeout)
    }

    pub fn stm32h5_lock<S: Adiv5Access>(cortex: &mut CortexM<S>) -> Result<(), AlgoError> {
        let _ = cortex.mem_write32(0x40022028, 1);
        Ok(())
    }

    pub fn flm_init<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        flash_addr: u32,
        clk: u32,
        func: u32,
    ) -> Result<(), AlgoError> {
        let entry = code_base + hdr.init_offset;
        let res = Self::exec_algo(
            cortex,
            entry,
            flash_addr,
            clk,
            func,
            0,
            hdr.stack_top,
            2000,
        )?;
        if res != 0 {
            return Err(AlgoError::ExecutionFailed(res));
        }
        Ok(())
    }

    pub fn flm_uninit<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        func: u32,
    ) -> Result<(), AlgoError> {
        if hdr.uninit_offset == 0 {
            return Ok(());
        }
        let entry = code_base + hdr.uninit_offset;
        let res = Self::exec_algo(
            cortex,
            entry,
            func,
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

    pub fn flm_program_turbo_double_buffered<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        target_flash_addr: u32,
        data: &[u8],
        page_size: u32,
    ) -> Result<(), AlgoError> {
        let buf0_addr = hdr.ram_base;
        let buf1_addr = hdr.ram_base + page_size;
        let entry = code_base + hdr.program_offset;

        let mut offset = 0usize;
        let first_chunk = (data.len() - offset).min(page_size as usize);

        Self::download_ram(cortex, buf0_addr, &data[offset..offset + first_chunk])?;

        Self::start_algo(
            cortex,
            entry,
            target_flash_addr + offset as u32,
            first_chunk as u32,
            buf0_addr,
            0,
            hdr.stack_top,
        )?;
        offset += first_chunk;
        let mut current_buf = 0;

        while offset < data.len() {
            let next_buf = 1 - current_buf;
            let next_buf_addr = if next_buf == 0 { buf0_addr } else { buf1_addr };
            let next_chunk = (data.len() - offset).min(page_size as usize);

            Self::download_ram(cortex, next_buf_addr, &data[offset..offset + next_chunk])?;

            let res = Self::wait_algo_halt(cortex, 2000)?;
            if res != 0 {
                return Err(AlgoError::ExecutionFailed(res));
            }

            Self::start_algo(
                cortex,
                entry,
                target_flash_addr + offset as u32,
                next_chunk as u32,
                next_buf_addr,
                0,
                hdr.stack_top,
            )?;

            offset += next_chunk;
            current_buf = next_buf;
        }

        let res = Self::wait_algo_halt(cortex, 2000)?;
        if res != 0 {
            return Err(AlgoError::ExecutionFailed(res));
        }

        Ok(())
    }

    pub fn flm_flash_blob<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        hdr: &AlgoBlobHeader,
        code: &[u8],
        target_ram_base: u32,
        target_flash_addr: u32,
        data: &[u8],
        sector_size: u32,
        page_size: u32,
    ) -> Result<(), AlgoError> {
        let code_base = target_ram_base;
        Self::download_ram(cortex, code_base, code)?;
        Self::flm_init(cortex, hdr, code_base, target_flash_addr, 0, 1)?;

        let mut curr_sector = target_flash_addr & !(sector_size - 1);
        let end_addr = target_flash_addr + data.len() as u32;
        while curr_sector < end_addr {
            Self::flm_erase_sector(cortex, hdr, code_base, curr_sector)?;
            curr_sector += sector_size;
        }

        let can_double_buffer = hdr.ram_size >= (2 * page_size) && page_size > 0;
        if can_double_buffer && data.len() > page_size as usize {
            Self::flm_program_turbo_double_buffered(
                cortex,
                hdr,
                code_base,
                target_flash_addr,
                data,
                page_size,
            )?;
        } else {
            let mut offset = 0usize;
            while offset < data.len() {
                let chunk_len = (data.len() - offset).min(page_size as usize);
                Self::flm_program_page(
                    cortex,
                    hdr,
                    code_base,
                    target_flash_addr + offset as u32,
                    &data[offset..offset + chunk_len],
                )?;
                offset += chunk_len;
            }
        }

        let ok = Self::flm_verify(cortex, hdr, code_base, target_flash_addr, data)?;
        if !ok {
            return Err(AlgoError::ExecutionFailed(1));
        }

        Self::flm_uninit(cortex, hdr, code_base, 2)?;
        Ok(())
    }

    pub fn flash_target<S: Adiv5Access>(
        cortex: &mut CortexM<S>,
        target: &TargetInfo,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        match target.family {
            TargetFamily::Stm32H5 => {
                let _ = cortex.select_ap(1);
                let sector_size = target.sector_size;
                let mut curr_sector = addr & !(sector_size - 1);
                let end_addr = addr + data.len() as u32;
                while curr_sector < end_addr {
                    Self::stm32h5_erase_sector(cortex, curr_sector)?;
                    curr_sector += sector_size;
                }
                let mut offset = 0usize;
                while offset < data.len() {
                    let chunk_len = (data.len() - offset).min(target.page_size as usize);
                    Self::stm32h5_program_page(
                        cortex,
                        addr + offset as u32,
                        &data[offset..offset + chunk_len],
                    )?;
                    offset += chunk_len;
                }
                let _ = Self::stm32h5_lock(cortex);
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
            TargetFamily::Stm32F1 | TargetFamily::Py32F0 => {
                let sector_size = target.sector_size;
                let mut curr_sector = addr & !(sector_size - 1);
                let end_addr = addr + data.len() as u32;
                while curr_sector < end_addr {
                    Self::stm32f1_erase_sector(cortex, curr_sector)?;
                    curr_sector += sector_size;
                }
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
