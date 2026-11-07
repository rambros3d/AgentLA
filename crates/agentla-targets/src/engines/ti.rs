//! Texas Instruments MSPM0 and SimpleLink (CC13x2, CC26x2) direct flash engines.

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const TI_CMD_ERASE: u32 = 0x01;
const TI_CMD_PROGRAM: u32 = 0x02;

fn ti_wait_busy<M: TargetMemAccess>(mem: &mut M, stat_addr: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(stat) = mem.read32(stat_addr) {
            if (stat & 0x01) == 0 { // Busy clear
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn mspm0_erase_sector<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    sector_addr: u32,
) -> Result<(), TargetError> {
    let cmd_addr = desc.reg_base + 0x00;
    let addr_reg = desc.reg_base + 0x04;
    let stat_addr = desc.reg_base + 0x08;

    mem.write32(addr_reg, sector_addr).map_err(|_| TargetError::BusError)?;
    mem.write32(cmd_addr, TI_CMD_ERASE).map_err(|_| TargetError::BusError)?;
    ti_wait_busy(mem, stat_addr)?;
    Ok(())
}

pub fn mspm0_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    let cmd_addr = desc.reg_base + 0x00;
    let addr_reg = desc.reg_base + 0x04;
    let stat_addr = desc.reg_base + 0x08;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word[..len].copy_from_slice(&data[offset..offset + len]);

        let curr_addr = addr + offset as u32;
        mem.write32(addr_reg, curr_addr).map_err(|_| TargetError::BusError)?;
        mem.write32(desc.reg_base + 0x0C, u32::from_le_bytes(word)).map_err(|_| TargetError::BusError)?;
        mem.write32(cmd_addr, TI_CMD_PROGRAM).map_err(|_| TargetError::BusError)?;

        ti_wait_busy(mem, stat_addr)?;
        offset += 4;
    }
    Ok(())
}
