pub mod blobs;

pub use blobs::{AlgoBlobHeader, STM32F1_ALGO_CODE, STM32F1_ALGO_HDR};
use crate::programmer::target::cortex_m::{CortexM, DHCSR, DHCSR_S_HALT};
use crate::programmer::target::targets::{TargetFamily, TargetInfo};
use core::fmt::Write;
use littlefs2::io::OpenSeekFrom;
use littlefs2::path::PathBuf;

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

    pub fn load_algo_from_fs(name: &str, buf: &mut [u8]) -> Result<(AlgoBlobHeader, usize), AlgoError> {
        let mut path_str = heapless::String::<64>::new();
        let _ = core::write!(path_str, "/system/algo/{}.bin", name);
        let path = PathBuf::try_from(path_str.as_str()).map_err(|_| AlgoError::AlgoNotFound)?;

        let read_result = crate::storage::with_system_fs(|fs| {
            if !fs.exists(&path) {
                return Err(AlgoError::AlgoNotFound);
            }
            let mut read_bytes = 0usize;
            let mut pos = 0u32;
            while read_bytes < buf.len() {
                let chunk_opt = fs.read_chunk::<512>(&path, OpenSeekFrom::Start(pos)).ok();
                if let Some((chunk, _total_len)) = chunk_opt {
                    if chunk.is_empty() {
                        break;
                    }
                    let to_copy = chunk.len().min(buf.len() - read_bytes);
                    buf[read_bytes..read_bytes + to_copy].copy_from_slice(&chunk[..to_copy]);
                    read_bytes += to_copy;
                    pos += to_copy as u32;
                    if chunk.len() < 512 {
                        break;
                    }
                } else {
                    break;
                }
            }
            Ok(read_bytes)
        });

        let bytes_read = match read_result {
            Some(Ok(n)) => n,
            Some(Err(e)) => return Err(e),
            None => return Err(AlgoError::AlgoNotFound),
        };

        if bytes_read < 44 {
            return Err(AlgoError::AlgoInvalid);
        }

        let hdr = AlgoBlobHeader::from_bytes(&buf[..44]).ok_or(AlgoError::AlgoInvalid)?;
        if 44 + hdr.code_size as usize > bytes_read {
            return Err(AlgoError::AlgoInvalid);
        }

        Ok((hdr, bytes_read))
    }

    pub fn flm_init(
        cortex: &mut CortexM,
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

    pub fn flm_uninit(
        cortex: &mut CortexM,
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

    pub fn flm_flash_blob(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code: &[u8],
        target_ram_base: u32,
        target_flash_addr: u32,
        data: &[u8],
        sector_size: u32,
        page_size: u32,
    ) -> Result<(), AlgoError> {
        let code_base = target_ram_base;
        // 1. Download algorithm code to target SRAM
        Self::download_ram(cortex, code_base, code)?;

        // 2. Call Init(target_flash_addr, 0, 1 = Erase)
        Self::flm_init(cortex, hdr, code_base, target_flash_addr, 0, 1)?;

        // 3. Erase sectors
        let mut curr_sector = target_flash_addr & !(sector_size - 1);
        let end_addr = target_flash_addr + data.len() as u32;
        while curr_sector < end_addr {
            Self::flm_erase_sector(cortex, hdr, code_base, curr_sector)?;
            curr_sector += sector_size;
        }

        // 4. Program pages
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

        // 5. Verify
        let ok = Self::flm_verify(cortex, hdr, code_base, target_flash_addr, data)?;
        if !ok {
            return Err(AlgoError::ExecutionFailed(1));
        }

        // 6. UnInit
        Self::flm_uninit(cortex, hdr, code_base, 2)?;
        Ok(())
    }

    pub fn flash_target_with_fs(
        cortex: &mut CortexM,
        algo_name: &str,
        target_ram_base: u32,
        target_flash_addr: u32,
        data: &[u8],
        sector_size: u32,
        page_size: u32,
    ) -> Result<(), AlgoError> {
        let mut algo_buf = [0u8; 4096];
        let (hdr, _len) = Self::load_algo_from_fs(algo_name, &mut algo_buf)?;
        let code = &algo_buf[44..44 + hdr.code_size as usize];
        Self::flm_flash_blob(cortex, &hdr, code, target_ram_base, target_flash_addr, data, sector_size, page_size)
    }

    // High level flash write + verify for any target
    pub fn flash_target(
        cortex: &mut CortexM,
        target: &TargetInfo,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        // 1. If device family has a matching LittleFS algorithm blob in /system/algo/, try running it
        if let Some(algo_name) = target.family.algo_name() {
            if let Ok(()) = Self::flash_target_with_fs(
                cortex,
                algo_name,
                target.ram_base,
                addr,
                data,
                target.sector_size,
                target.page_size,
            ) {
                return Ok(());
            }
        }

        // 2. Hardware-specific fallback or built-in algorithm
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
