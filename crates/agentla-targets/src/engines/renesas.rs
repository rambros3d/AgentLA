//! Renesas RA2, RA4, RA6 Cortex-M flash controller engine.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const RA_FENTRYR: u32 = 0x00;
const RA_FPESTAT: u32 = 0x0C;

fn ra_wait_ready<M: TargetMemAccess>(mem: &mut M, base: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(stat) = mem.read32(base + RA_FPESTAT) {
            if (stat & 0x01) == 0 {
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn ra_erase_block<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    block_addr: u32,
) -> Result<(), TargetError> {
    // Enter PE mode: FENTRYR = 0xAA01
    mem.write32(desc.reg_base + RA_FENTRYR, 0xAA01).map_err(|_| TargetError::BusError)?;

    // Execute block erase command (write 0x20 then 0xD0 to block address)
    mem.write32(block_addr, 0x20).map_err(|_| TargetError::BusError)?;
    mem.write32(block_addr, 0xD0).map_err(|_| TargetError::BusError)?;

    ra_wait_ready(mem, desc.reg_base)?;

    // Exit PE mode: FENTRYR = 0xAA00
    mem.write32(desc.reg_base + RA_FENTRYR, 0xAA00).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn ra_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    mem.write32(desc.reg_base + RA_FENTRYR, 0xAA01).map_err(|_| TargetError::BusError)?;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word[..len].copy_from_slice(&data[offset..offset + len]);

        let curr_addr = addr + offset as u32;
        mem.write32(curr_addr, 0xE8).map_err(|_| TargetError::BusError)?; // Program setup
        mem.write32(curr_addr, u32::from_le_bytes(word)).map_err(|_| TargetError::BusError)?;
        mem.write32(curr_addr, 0xD0).map_err(|_| TargetError::BusError)?; // Confirm

        ra_wait_ready(mem, desc.reg_base)?;
        offset += 4;
    }

    mem.write32(desc.reg_base + RA_FENTRYR, 0xAA00).map_err(|_| TargetError::BusError)?;
    Ok(())
}
