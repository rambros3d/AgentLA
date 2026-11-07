//! Microchip SAMD, SAMC, SAML, SAME, SAMV, and PIC32CM direct flash engines.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const SAMD_NVM_CTRLA: u32 = 0x00;
const SAMD_NVM_INTFLAG: u32 = 0x14;
const SAMD_CMDEX: u32 = 0xA500;
const SAMD_CMD_ER: u32 = 0x02; // Erase Row (4 pages)
const SAMD_CMD_WP: u32 = 0x04; // Write Page
const SAMD_CMD_PBC: u32 = 0x44; // Page Buffer Clear

fn samd_wait_ready<M: TargetMemAccess>(mem: &mut M, base: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(flags) = mem.read32(base + SAMD_NVM_INTFLAG) {
            if (flags & 0x01) != 0 {
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn samd_erase_row<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    row_addr: u32,
) -> Result<(), TargetError> {
    samd_wait_ready(mem, desc.reg_base)?;

    // Dummy write to the row address to set target ADDR in NVMCTRL
    let _ = mem.write32(row_addr, 0xFFFFFFFF);

    // Issue Erase Row command
    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | SAMD_CMD_ER)
        .map_err(|_| TargetError::BusError)?;

    samd_wait_ready(mem, desc.reg_base)?;
    Ok(())
}

pub fn samd_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    samd_wait_ready(mem, desc.reg_base)?;

    // Clear internal page buffer
    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | SAMD_CMD_PBC)
        .map_err(|_| TargetError::BusError)?;
    samd_wait_ready(mem, desc.reg_base)?;

    // Fill page buffer via 32-bit word writes
    let mut offset = 0usize;
    while offset < data.len() {
        let mut word_bytes = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word_bytes[..len].copy_from_slice(&data[offset..offset + len]);
        let word = u32::from_le_bytes(word_bytes);

        mem.write32(addr + offset as u32, word)
            .map_err(|_| TargetError::BusError)?;
        offset += 4;
    }

    // Trigger Write Page
    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | SAMD_CMD_WP)
        .map_err(|_| TargetError::BusError)?;
    samd_wait_ready(mem, desc.reg_base)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// SAME51/E54 / SAMV71 Engine
// ----------------------------------------------------------------------------
pub fn same_erase_block<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    block_addr: u32,
) -> Result<(), TargetError> {
    samd_wait_ready(mem, desc.reg_base)?;
    let _ = mem.write32(block_addr, 0xFFFFFFFF);
    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | 0x02) // Erase Block
        .map_err(|_| TargetError::BusError)?;
    samd_wait_ready(mem, desc.reg_base)?;
    Ok(())
}

pub fn same_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    samd_wait_ready(mem, desc.reg_base)?;
    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | 0x15) // Page buffer clear
        .map_err(|_| TargetError::BusError)?;
    samd_wait_ready(mem, desc.reg_base)?;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word_bytes = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word_bytes[..len].copy_from_slice(&data[offset..offset + len]);
        let word = u32::from_le_bytes(word_bytes);
        mem.write32(addr + offset as u32, word)
            .map_err(|_| TargetError::BusError)?;
        offset += 4;
    }

    mem.write32(desc.reg_base + SAMD_NVM_CTRLA, SAMD_CMDEX | 0x03) // Write Page
        .map_err(|_| TargetError::BusError)?;
    samd_wait_ready(mem, desc.reg_base)?;
    Ok(())
}
