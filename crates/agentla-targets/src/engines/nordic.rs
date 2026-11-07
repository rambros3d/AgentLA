//! Nordic NVMC direct flash engine (nRF51, nRF52, nRF53, nRF91).

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const NVMC_READY: u32 = 0x400;
const NVMC_CONFIG: u32 = 0x504;
const NVMC_ERASEPAGE: u32 = 0x508;

fn wait_ready<M: TargetMemAccess>(mem: &mut M, base: u32) -> Result<(), TargetError> {
    let ready_addr = base + NVMC_READY;
    for _ in 0..100_000 {
        if let Ok(ready) = mem.read32(ready_addr) {
            if (ready & 0x01) != 0 {
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn nvmc_erase_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    page_addr: u32,
) -> Result<(), TargetError> {
    wait_ready(mem, desc.reg_base)?;

    // Enable erase: CONFIG = 2 (EEN)
    mem.write32(desc.reg_base + NVMC_CONFIG, 0x02)
        .map_err(|_| TargetError::BusError)?;

    // Trigger page erase
    mem.write32(desc.reg_base + NVMC_ERASEPAGE, page_addr)
        .map_err(|_| TargetError::BusError)?;

    wait_ready(mem, desc.reg_base)?;

    // Return to read-only: CONFIG = 0 (REN)
    mem.write32(desc.reg_base + NVMC_CONFIG, 0x00)
        .map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn nvmc_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    wait_ready(mem, desc.reg_base)?;

    // Enable write: CONFIG = 1 (WEN)
    mem.write32(desc.reg_base + NVMC_CONFIG, 0x01)
        .map_err(|_| TargetError::BusError)?;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word_bytes = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word_bytes[..len].copy_from_slice(&data[offset..offset + len]);
        let word = u32::from_le_bytes(word_bytes);

        mem.write32(addr + offset as u32, word)
            .map_err(|_| TargetError::BusError)?;
        wait_ready(mem, desc.reg_base)?;
        offset += 4;
    }

    // Return to read-only: CONFIG = 0
    mem.write32(desc.reg_base + NVMC_CONFIG, 0x00)
        .map_err(|_| TargetError::BusError)?;
    Ok(())
}
