//! RISC-V direct flash engines: WCH QingKe (V2 1-wire, V4 2-wire, BLE), Espressif SPI, GD32V.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

// WCH Fast Flash Registers
const WCH_FLASH_KEYR: u32 = 0x04;
const WCH_FLASH_CR: u32 = 0x10;
const WCH_FLASH_SR: u32 = 0x0C;

const WCH_KEY1: u32 = 0x45670123;
const WCH_KEY2: u32 = 0xCDEF89AB;

fn wch_unlock<M: TargetMemAccess>(mem: &mut M, reg_base: u32) -> Result<(), TargetError> {
    mem.write32(reg_base + WCH_FLASH_KEYR, WCH_KEY1).map_err(|_| TargetError::BusError)?;
    mem.write32(reg_base + WCH_FLASH_KEYR, WCH_KEY2).map_err(|_| TargetError::BusError)?;
    Ok(())
}

fn wch_wait_bsy<M: TargetMemAccess>(mem: &mut M, reg_base: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(sr) = mem.read32(reg_base + WCH_FLASH_SR) {
            if (sr & 0x01) == 0 { // BSY bit 0 clear
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

// ----------------------------------------------------------------------------
// 1. WCH QingKe V2 (CH32V003 1-Wire SWIO) Fast Page Engine
// ----------------------------------------------------------------------------
pub fn qingke_v2_erase_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    page_addr: u32,
) -> Result<(), TargetError> {
    wch_unlock(mem, desc.reg_base)?;

    // Set PER bit and page address in AR
    mem.write32(desc.reg_base + WCH_FLASH_CR, 0x02).map_err(|_| TargetError::BusError)?;
    mem.write32(desc.reg_base + 0x14, page_addr).map_err(|_| TargetError::BusError)?;
    mem.write32(desc.reg_base + WCH_FLASH_CR, 0x02 | 0x40).map_err(|_| TargetError::BusError)?; // STRT

    wch_wait_bsy(mem, desc.reg_base)?;
    mem.write32(desc.reg_base + WCH_FLASH_CR, 0x00).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn qingke_v2_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    wch_unlock(mem, desc.reg_base)?;

    // 64-byte fast page write loop
    let mut offset = 0usize;
    while offset < data.len() {
        let halfword = if offset + 1 < data.len() {
            (data[offset] as u16) | ((data[offset + 1] as u16) << 8)
        } else {
            (data[offset] as u16) | 0xFF00
        };

        mem.write32(desc.reg_base + WCH_FLASH_CR, 0x01).map_err(|_| TargetError::BusError)?; // PG=1
        let curr_addr = addr + offset as u32;

        let aligned_addr = curr_addr & !3;
        let shift = (curr_addr & 2) * 8;
        let orig = mem.read32(aligned_addr).unwrap_or(0xFFFFFFFF);
        let mask = !(0xFFFF << shift);
        let new_word = (orig & mask) | ((halfword as u32) << shift);
        mem.write32(aligned_addr, new_word).map_err(|_| TargetError::BusError)?;

        wch_wait_bsy(mem, desc.reg_base)?;
        offset += 2;
    }
    mem.write32(desc.reg_base + WCH_FLASH_CR, 0x00).map_err(|_| TargetError::BusError)?;
    Ok(())
}

// ----------------------------------------------------------------------------
// 2. WCH QingKe V4 (CH32V103, V203, V307 2-Wire RVSWD)
// ----------------------------------------------------------------------------
pub fn qingke_v4_erase_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    page_addr: u32,
) -> Result<(), TargetError> {
    qingke_v2_erase_page(mem, desc, page_addr)
}

pub fn qingke_v4_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    qingke_v2_program_page(mem, desc, addr, data)
}

// ----------------------------------------------------------------------------
// 3. WCH BLE Series (CH571, CH573, CH582, CH592)
// ----------------------------------------------------------------------------
pub fn wch_ble_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    qingke_v2_erase_page(mem, desc, sector_addr)
}

pub fn wch_ble_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    qingke_v2_program_page(mem, desc, addr, data)
}

// ----------------------------------------------------------------------------
// 4. Espressif RISC-V Direct SPI Flash Controller (ESP32-C2/C3/C6/H2)
// ----------------------------------------------------------------------------
pub fn esp32_spi_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    // Direct SPI1 flash controller registers at 0x60002000
    let spi_cmd_reg = desc.reg_base + 0x00;
    let spi_addr_reg = desc.reg_base + 0x04;

    mem.write32(spi_addr_reg, sector_addr).map_err(|_| TargetError::BusError)?;
    mem.write32(spi_cmd_reg, 1 << 2).map_err(|_| TargetError::BusError)?; // FLASH_SE bit 2 (Sector Erase)

    for _ in 0..100_000 {
        if let Ok(cmd) = mem.read32(spi_cmd_reg) {
            if (cmd & (1 << 2)) == 0 { // Done
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn esp32_spi_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    let spi_cmd_reg = desc.reg_base + 0x00;
    let spi_addr_reg = desc.reg_base + 0x04;
    let spi_w0_reg = desc.reg_base + 0x58; // SPI Data Buffer W0

    let mut offset = 0usize;
    while offset < data.len() {
        let chunk_len = (data.len() - offset).min(64); // 64-byte burst write
        for i in 0..(chunk_len / 4) {
            let w = u32::from_le_bytes(data[offset + i*4..offset + (i+1)*4].try_into().unwrap());
            mem.write32(spi_w0_reg + (i * 4) as u32, w).map_err(|_| TargetError::BusError)?;
        }

        let curr_addr = addr + offset as u32;
        mem.write32(spi_addr_reg, curr_addr).map_err(|_| TargetError::BusError)?;
        mem.write32(spi_cmd_reg, 1 << 1).map_err(|_| TargetError::BusError)?; // FLASH_PP (Page Program)

        for _ in 0..50_000 {
            if let Ok(cmd) = mem.read32(spi_cmd_reg) {
                if (cmd & (1 << 1)) == 0 {
                    break;
                }
            }
        }
        offset += chunk_len;
    }
    Ok(())
}

// ----------------------------------------------------------------------------
// 5. GigaDevice GD32VF103 RISC-V Engine
// ----------------------------------------------------------------------------
pub fn gd32v_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    qingke_v2_erase_page(mem, desc, sector_addr)
}

pub fn gd32v_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    qingke_v2_program_page(mem, desc, addr, data)
}
