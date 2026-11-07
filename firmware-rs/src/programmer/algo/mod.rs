//! CMSIS FLM flash algorithm execution engine and direct flash controllers.

pub mod blobs;

pub use agentla_programmer::algo::*;
pub use blobs::{AlgoBlobHeader, STM32F1_ALGO_CODE, STM32F1_ALGO_HDR};

use crate::programmer::target::cortex_m::CortexM;
use crate::programmer::target::targets::TargetInfo;
use core::fmt::Write;
use littlefs2::io::OpenSeekFrom;
use littlefs2::path::PathBuf;

pub struct FlashAlgorithmRunner;

impl FlashAlgorithmRunner {
    #[inline(always)]
    pub fn download_ram(cortex: &mut CortexM, addr: u32, data: &[u8]) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::download_ram(cortex, addr, data)
    }

    #[inline(always)]
    pub fn start_algo(
        cortex: &mut CortexM,
        entry: u32,
        r0: u32,
        r1: u32,
        r2: u32,
        r3: u32,
        sp: u32,
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::start_algo(cortex, entry, r0, r1, r2, r3, sp)
    }

    #[inline(always)]
    pub fn wait_algo_halt(cortex: &mut CortexM, timeout_ms: u32) -> Result<u32, AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::wait_algo_halt(cortex, timeout_ms)
    }

    #[inline(always)]
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
        agentla_programmer::algo::FlashAlgorithmRunner::exec_algo(cortex, entry, r0, r1, r2, r3, sp, timeout_ms)
    }

    #[inline(always)]
    pub fn flm_erase_sector(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        sector_addr: u32,
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_erase_sector(cortex, hdr, code_base, sector_addr)
    }

    #[inline(always)]
    pub fn flm_program_page(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        page_addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_program_page(cortex, hdr, code_base, page_addr, data)
    }

    #[inline(always)]
    pub fn flm_verify(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        addr: u32,
        data: &[u8],
    ) -> Result<bool, AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_verify(cortex, hdr, code_base, addr, data)
    }

    #[inline(always)]
    pub fn stm32f1_unlock(cortex: &mut CortexM) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::stm32f1_unlock(cortex)
    }

    #[inline(always)]
    pub fn stm32f1_wait_bsy(cortex: &mut CortexM) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::stm32f1_wait_bsy(cortex)
    }

    #[inline(always)]
    pub fn stm32f1_erase_sector(cortex: &mut CortexM, sector_addr: u32) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::stm32f1_erase_sector(cortex, sector_addr)
    }

    #[inline(always)]
    pub fn stm32f1_program_page(cortex: &mut CortexM, addr: u32, data: &[u8]) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::stm32f1_program_page(cortex, addr, data)
    }

    #[inline(always)]
    pub fn flm_init(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        flash_addr: u32,
        clk: u32,
        func: u32,
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_init(cortex, hdr, code_base, flash_addr, clk, func)
    }

    #[inline(always)]
    pub fn flm_uninit(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        func: u32,
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_uninit(cortex, hdr, code_base, func)
    }

    #[inline(always)]
    pub fn flm_program_turbo_double_buffered(
        cortex: &mut CortexM,
        hdr: &AlgoBlobHeader,
        code_base: u32,
        target_flash_addr: u32,
        data: &[u8],
        page_size: u32,
    ) -> Result<(), AlgoError> {
        agentla_programmer::algo::FlashAlgorithmRunner::flm_program_turbo_double_buffered(
            cortex,
            hdr,
            code_base,
            target_flash_addr,
            data,
            page_size,
        )
    }

    #[inline(always)]
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
        agentla_programmer::algo::FlashAlgorithmRunner::flm_flash_blob(
            cortex,
            hdr,
            code,
            target_ram_base,
            target_flash_addr,
            data,
            sector_size,
            page_size,
        )
    }

    pub fn load_algo_from_fs(name: &str, buf: &mut [u8]) -> Result<(AlgoBlobHeader, usize), AlgoError> {
        let mut path_str = heapless::String::<64>::new();
        let _ = write!(path_str, "/system/algo/{}.bin", name);
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

    pub fn flash_target(
        cortex: &mut CortexM,
        target: &TargetInfo,
        addr: u32,
        data: &[u8],
    ) -> Result<(), AlgoError> {
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
        agentla_programmer::algo::FlashAlgorithmRunner::flash_target(cortex, target, addr, data)
    }
}
