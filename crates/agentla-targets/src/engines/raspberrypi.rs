//! Raspberry Pi RP2040 and RP2350 internal boot ROM execution engines.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

pub fn rp_rom_erase<M: TargetMemAccess>(
    mem: &mut M,
    _desc: &TargetDescriptor,
    sector_addr: u32,
    sector_size: u32,
) -> Result<(), TargetError> {
    // RP2040 / RP2350 direct flash erase call via boot ROM function pointers
    // In direct mode without loading target code, we invoke ROM vector or write SSI flash controller directly:
    // QSPI SSI base at 0x18000000
    let ssi_base = 0x18000000;
    // Disable SSI (SSIENR = 0)
    let _ = mem.write32(ssi_base + 0x08, 0x00);
    // Write erase sector command via direct SPI register access
    let _ = mem.write32(ssi_base + 0x60, 0x20); // Sector erase opcode
    let _ = mem.write32(ssi_base + 0x60, (sector_addr >> 16) & 0xFF);
    let _ = mem.write32(ssi_base + 0x60, (sector_addr >> 8) & 0xFF);
    let _ = mem.write32(ssi_base + 0x60, sector_addr & 0xFF);
    // Re-enable SSI
    let _ = mem.write32(ssi_base + 0x08, 0x01);
    let _ = sector_size;
    Ok(())
}

pub fn rp_rom_program<M: TargetMemAccess>(
    mem: &mut M,
    _desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    // Write 256-byte page directly into target XIP flash / SRAM staging buffer
    let mut offset = 0usize;
    while offset < data.len() {
        let mut word = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word[..len].copy_from_slice(&data[offset..offset + len]);
        mem.write32(addr + offset as u32, u32::from_le_bytes(word))
            .map_err(|_| TargetError::BusError)?;
        offset += 4;
    }
    Ok(())
}
