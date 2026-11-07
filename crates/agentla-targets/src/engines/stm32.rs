//! STM32 family direct flash engines: Classic 16-bit, Sector-based (F2/F4/F7), Modern 64-bit, and Dual-Bank H7.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

// STM32 Classic Registers (F1/F3, GD32, APM32, AT32)
const FLASH_KEYR_OFFSET: u32 = 0x04;
const FLASH_SR_OFFSET: u32 = 0x0C;
const FLASH_CR_OFFSET: u32 = 0x10;
const FLASH_AR_OFFSET: u32 = 0x14;

const CR_PG: u32 = 1 << 0;
const CR_PER: u32 = 1 << 1;
#[allow(dead_code)]
const CR_MER: u32 = 1 << 2;
const CR_STRT: u32 = 1 << 6;
const CR_LOCK: u32 = 1 << 7;

const SR_BSY: u32 = 1 << 0;
const SR_PGERR: u32 = 1 << 2;
const SR_WRPRTERR: u32 = 1 << 4;
const SR_EOP: u32 = 1 << 5;

fn wait_bsy<M: TargetMemAccess>(mem: &mut M, sr_addr: u32, timeout_cycles: u32) -> Result<(), TargetError> {
    for _ in 0..timeout_cycles {
        let sr = mem.read32(sr_addr).map_err(|_| TargetError::BusError)?;
        if (sr & SR_BSY) == 0 {
            if (sr & (SR_PGERR | SR_WRPRTERR)) != 0 {
                // Clear errors
                let _ = mem.write32(sr_addr, SR_PGERR | SR_WRPRTERR | SR_EOP);
                return Err(TargetError::FlashControllerFault(sr));
            }
            return Ok(());
        }
    }
    Err(TargetError::Timeout)
}

fn unlock<M: TargetMemAccess>(mem: &mut M, desc: &TargetDescriptor) -> Result<(), TargetError> {
    let cr_addr = desc.reg_base + FLASH_CR_OFFSET;
    let cr = mem.read32(cr_addr).map_err(|_| TargetError::BusError)?;
    if (cr & CR_LOCK) != 0 {
        let keyr = desc.reg_base + FLASH_KEYR_OFFSET;
        mem.write32(keyr, desc.keys.0).map_err(|_| TargetError::BusError)?;
        mem.write32(keyr, desc.keys.1).map_err(|_| TargetError::BusError)?;
        let cr_after = mem.read32(cr_addr).map_err(|_| TargetError::BusError)?;
        if (cr_after & CR_LOCK) != 0 {
            return Err(TargetError::UnlockFailed);
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// 1. STM32 Classic 16-Bit Engine (F1/F3, GD32, APM32, AT32)
// ----------------------------------------------------------------------------
pub fn classic16_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + FLASH_CR_OFFSET;
    let ar_addr = desc.reg_base + FLASH_AR_OFFSET;
    let sr_addr = desc.reg_base + FLASH_SR_OFFSET;

    // Set PER, write sector address to AR, assert STRT
    mem.write32(cr_addr, CR_PER).map_err(|_| TargetError::BusError)?;
    mem.write32(ar_addr, sector_addr).map_err(|_| TargetError::BusError)?;
    mem.write32(cr_addr, CR_PER | CR_STRT).map_err(|_| TargetError::BusError)?;

    wait_bsy(mem, sr_addr, 100_000)?;
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn classic16_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + FLASH_CR_OFFSET;
    let sr_addr = desc.reg_base + FLASH_SR_OFFSET;

    let mut offset = 0usize;
    while offset < data.len() {
        let halfword = if offset + 1 < data.len() {
            (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
        } else {
            (data[offset] as u16) | 0xFF00
        };

        mem.write32(cr_addr, CR_PG).map_err(|_| TargetError::BusError)?;
        let curr_addr = addr + offset as u32;

        // Perform 16-bit halfword write via TargetMemAccess word read-modify-write or native write
        let aligned_addr = curr_addr & !3;
        let shift = (curr_addr & 2) * 8;
        let orig = mem.read32(aligned_addr).unwrap_or(0xFFFFFFFF);
        let mask = !(0xFFFF << shift);
        let new_word = (orig & mask) | ((halfword as u32) << shift);
        mem.write32(aligned_addr, new_word).map_err(|_| TargetError::BusError)?;

        wait_bsy(mem, sr_addr, 10_000)?;
        offset += 2;
    }
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// 2. STM32 Sector-Based Engine (F2/F4/F7: 16/64/128 KB Sectors, 32-bit Program)
// ----------------------------------------------------------------------------
pub fn sector_erase<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + FLASH_CR_OFFSET;
    let sr_addr = desc.reg_base + FLASH_SR_OFFSET;

    // Calculate sector number from sector_addr relative to flash_base
    let offset = sector_addr.saturating_sub(desc.flash_base);
    let snb = if offset < 0x10000 {
        offset / 0x4000 // 16 KB sectors 0-3
    } else if offset < 0x20000 {
        4 // 64 KB sector 4
    } else {
        5 + (offset - 0x20000) / 0x20000 // 128 KB sectors 5+
    };

    let cr_val = (1 << 1) | ((snb & 0x1F) << 3) | (2 << 8); // SER=1, SNB, PSIZE=32-bit
    mem.write32(cr_addr, cr_val).map_err(|_| TargetError::BusError)?;
    mem.write32(cr_addr, cr_val | (1 << 16)).map_err(|_| TargetError::BusError)?; // STRT=1 (bit 16 in F4)

    wait_bsy(mem, sr_addr, 200_000)?;
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn sector_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + FLASH_CR_OFFSET;
    let sr_addr = desc.reg_base + FLASH_SR_OFFSET;

    let cr_pg = (1 << 0) | (2 << 8); // PG=1, PSIZE=32-bit
    mem.write32(cr_addr, cr_pg).map_err(|_| TargetError::BusError)?;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word_bytes = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word_bytes[..len].copy_from_slice(&data[offset..offset + len]);
        let word = u32::from_le_bytes(word_bytes);

        mem.write32(addr + offset as u32, word).map_err(|_| TargetError::BusError)?;
        wait_bsy(mem, sr_addr, 10_000)?;
        offset += 4;
    }
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// 3. STM32 Modern 64-Bit Engine (G0/G4/L0/L4/C0/U5/H5)
// ----------------------------------------------------------------------------
pub fn modern64_erase_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    page_addr: u32,
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + 0x14; // CR in modern series
    let sr_addr = desc.reg_base + 0x10; // SR in modern series

    let page_num = (page_addr.saturating_sub(desc.flash_base)) / (desc.page_size as u32);
    let cr_val = (1 << 1) | ((page_num & 0x1FF) << 3) | (1 << 16); // PER=1, PNB, STRT=bit 16
    mem.write32(cr_addr, cr_val).map_err(|_| TargetError::BusError)?;

    wait_bsy(mem, sr_addr, 100_000)?;
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn modern64_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    unlock(mem, desc)?;
    let cr_addr = desc.reg_base + 0x14;
    let sr_addr = desc.reg_base + 0x10;

    mem.write32(cr_addr, 1 << 0).map_err(|_| TargetError::BusError)?; // PG=1

    // Program strictly as 64-bit double-words
    let mut offset = 0usize;
    while offset < data.len() {
        let mut dword = [0xFFu8; 8];
        let len = (data.len() - offset).min(8);
        dword[..len].copy_from_slice(&data[offset..offset + len]);

        let w0 = u32::from_le_bytes(dword[0..4].try_into().unwrap());
        let w1 = u32::from_le_bytes(dword[4..8].try_into().unwrap());

        mem.write32(addr + offset as u32, w0).map_err(|_| TargetError::BusError)?;
        mem.write32(addr + offset as u32 + 4, w1).map_err(|_| TargetError::BusError)?;

        wait_bsy(mem, sr_addr, 15_000)?;
        offset += 8;
    }
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// 4. STM32 Dual-Bank High-Performance Engine (STM32H7)
// ----------------------------------------------------------------------------
pub fn h7_dualbank_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    // STM32H7 bank 1 / bank 2 split at 1MB offset
    let is_bank2 = sector_addr >= (desc.flash_base + 0x100000);
    let bank_offset = if is_bank2 { 0x100 } else { 0x00 };
    let cr_addr = desc.reg_base + bank_offset + 0x0C;
    let sr_addr = desc.reg_base + bank_offset + 0x10;

    // Unlock bank
    let keyr_addr = desc.reg_base + bank_offset + 0x04;
    let _ = mem.write32(keyr_addr, 0x45670123);
    let _ = mem.write32(keyr_addr, 0xCDEF89AB);

    let sector_num = (sector_addr & 0x0FFFFF) / desc.sector_size;
    let cr_val = (1 << 2) | ((sector_num & 0x07) << 4) | (1 << 7); // SER=1, SNB, START=bit 7
    mem.write32(cr_addr, cr_val).map_err(|_| TargetError::BusError)?;

    wait_bsy(mem, sr_addr, 200_000)?;
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn h7_dualbank_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    let is_bank2 = addr >= (desc.flash_base + 0x100000);
    let bank_offset = if is_bank2 { 0x100 } else { 0x00 };
    let cr_addr = desc.reg_base + bank_offset + 0x0C;
    let sr_addr = desc.reg_base + bank_offset + 0x10;

    // H7 programs in 256-bit (32-byte) flash lines
    mem.write32(cr_addr, 1 << 1).map_err(|_| TargetError::BusError)?; // PG=1

    let mut offset = 0usize;
    while offset < data.len() {
        let mut line = [0xFFu8; 32];
        let len = (data.len() - offset).min(32);
        line[..len].copy_from_slice(&data[offset..offset + len]);

        for i in 0..8 {
            let w = u32::from_le_bytes(line[i*4..(i+1)*4].try_into().unwrap());
            mem.write32(addr + offset as u32 + (i * 4) as u32, w)
                .map_err(|_| TargetError::BusError)?;
        }

        wait_bsy(mem, sr_addr, 20_000)?;
        offset += 32;
    }
    mem.write32(cr_addr, 0).map_err(|_| TargetError::BusError)?;
    Ok(())
}
