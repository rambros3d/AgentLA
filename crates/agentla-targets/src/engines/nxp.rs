//! NXP LPC and Kinetis direct flash engines.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

// NXP Kinetis FTFL Registers
const FTFL_FSTAT: u32 = 0x00;
const FTFL_FCCOB3: u32 = 0x04;

const FSTAT_CCIF: u32 = 1 << 7;
const FSTAT_ACCERR: u32 = 1 << 5;
const FSTAT_FPVIOL: u32 = 1 << 4;

fn kinetis_wait_ccif<M: TargetMemAccess>(mem: &mut M, base: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(fstat) = mem.read32(base + FTFL_FSTAT) {
            if (fstat & FSTAT_CCIF) != 0 {
                if (fstat & (FSTAT_ACCERR | FSTAT_FPVIOL)) != 0 {
                    // Clear error flags
                    let _ = mem.write32(base + FTFL_FSTAT, FSTAT_ACCERR | FSTAT_FPVIOL);
                    return Err(TargetError::FlashControllerFault(fstat));
                }
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn kinetis_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    kinetis_wait_ccif(mem, desc.reg_base)?;

    // Clear previous errors if any
    let _ = mem.write32(desc.reg_base + FTFL_FSTAT, FSTAT_ACCERR | FSTAT_FPVIOL);

    // Command 0x09: Erase Flash Sector
    // FCCOB0 = 0x09, FCCOB1 = addr[23:16], FCCOB2 = addr[15:8], FCCOB3 = addr[7:0]
    let word0 = 0x09 | (((sector_addr >> 16) & 0xFF) << 8) | (((sector_addr >> 8) & 0xFF) << 16) | ((sector_addr & 0xFF) << 24);
    mem.write32(desc.reg_base + FTFL_FCCOB3, word0).map_err(|_| TargetError::BusError)?;

    // Launch command: write 1 to CCIF to clear it
    mem.write32(desc.reg_base + FTFL_FSTAT, FSTAT_CCIF).map_err(|_| TargetError::BusError)?;
    kinetis_wait_ccif(mem, desc.reg_base)?;
    Ok(())
}

pub fn kinetis_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    let mut offset = 0usize;
    while offset < data.len() {
        kinetis_wait_ccif(mem, desc.reg_base)?;

        let curr_addr = addr + offset as u32;
        let mut dword = [0xFFu8; 8];
        let len = (data.len() - offset).min(8);
        dword[..len].copy_from_slice(&data[offset..offset + len]);

        // Command 0x06: Program Flash (or 0x07 Program Section)
        let word0 = 0x06 | (((curr_addr >> 16) & 0xFF) << 8) | (((curr_addr >> 8) & 0xFF) << 16) | ((curr_addr & 0xFF) << 24);
        let word1 = u32::from_le_bytes(dword[0..4].try_into().unwrap());
        let word2 = u32::from_le_bytes(dword[4..8].try_into().unwrap());

        mem.write32(desc.reg_base + FTFL_FCCOB3, word0).map_err(|_| TargetError::BusError)?;
        mem.write32(desc.reg_base + FTFL_FCCOB3 + 4, word1).map_err(|_| TargetError::BusError)?;
        mem.write32(desc.reg_base + FTFL_FCCOB3 + 8, word2).map_err(|_| TargetError::BusError)?;

        mem.write32(desc.reg_base + FTFL_FSTAT, FSTAT_CCIF).map_err(|_| TargetError::BusError)?;
        kinetis_wait_ccif(mem, desc.reg_base)?;
        offset += 8;
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// NXP LPC Command-Based Flash Engine (LPC800, LPC1100, LPC54000)
// ----------------------------------------------------------------------------
pub fn lpc_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    // LPC FMC Command sequence: set start and stop sector
    let sector_num = sector_addr / desc.sector_size;
    mem.write32(desc.reg_base + 0x00, sector_num).map_err(|_| TargetError::BusError)?; // FMSSTART
    mem.write32(desc.reg_base + 0x04, sector_num).map_err(|_| TargetError::BusError)?; // FMSSTOP
    mem.write32(desc.reg_base + 0x08, 0x02).map_err(|_| TargetError::BusError)?; // ERASE CMD

    for _ in 0..100_000 {
        if let Ok(stat) = mem.read32(desc.reg_base + 0x0C) {
            if (stat & 0x04) != 0 { // Done
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn lpc_program_page<M: TargetMemAccess>(
    mem: &mut M,
    _desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    // Write direct words to target flash buffer
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
