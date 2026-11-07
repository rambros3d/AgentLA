//! Puya PY32F0 ultra-low-power Cortex-M0+ flash controller engine.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const PY32_CR_OFFSET: u32 = 0x10;
const PY32_SR_OFFSET: u32 = 0x0C;
const PY32_KEYR_OFFSET: u32 = 0x04;

pub fn py32_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    // Unlock flash
    let _ = mem.write32(desc.reg_base + PY32_KEYR_OFFSET, desc.keys.0);
    let _ = mem.write32(desc.reg_base + PY32_KEYR_OFFSET, desc.keys.1);

    // Set PER=1 (bit 1) and address
    mem.write32(desc.reg_base + PY32_CR_OFFSET, 0x02).map_err(|_| TargetError::BusError)?;
    mem.write32(desc.reg_base + 0x14, sector_addr).map_err(|_| TargetError::BusError)?;
    mem.write32(desc.reg_base + PY32_CR_OFFSET, 0x02 | 0x40).map_err(|_| TargetError::BusError)?; // STRT bit 6

    for _ in 0..50_000 {
        if let Ok(sr) = mem.read32(desc.reg_base + PY32_SR_OFFSET) {
            if (sr & 0x01) == 0 { // BSY clear
                mem.write32(desc.reg_base + PY32_CR_OFFSET, 0x00).map_err(|_| TargetError::BusError)?;
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn py32_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    let _ = mem.write32(desc.reg_base + PY32_KEYR_OFFSET, desc.keys.0);
    let _ = mem.write32(desc.reg_base + PY32_KEYR_OFFSET, desc.keys.1);

    let mut offset = 0usize;
    while offset < data.len() {
        let halfword = if offset + 1 < data.len() {
            (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
        } else {
            (data[offset] as u16) | 0xFF00
        };

        mem.write32(desc.reg_base + PY32_CR_OFFSET, 0x01).map_err(|_| TargetError::BusError)?; // PG bit 0
        let curr_addr = addr + offset as u32;

        let aligned_addr = curr_addr & !3;
        let shift = (curr_addr & 2) * 8;
        let orig = mem.read32(aligned_addr).unwrap_or(0xFFFFFFFF);
        let mask = !(0xFFFF << shift);
        let new_word = (orig & mask) | ((halfword as u32) << shift);
        mem.write32(aligned_addr, new_word).map_err(|_| TargetError::BusError)?;

        for _ in 0..10_000 {
            if let Ok(sr) = mem.read32(desc.reg_base + PY32_SR_OFFSET) {
                if (sr & 0x01) == 0 {
                    break;
                }
            }
        }
        offset += 2;
    }
    mem.write32(desc.reg_base + PY32_CR_OFFSET, 0x00).map_err(|_| TargetError::BusError)?;
    Ok(())
}
