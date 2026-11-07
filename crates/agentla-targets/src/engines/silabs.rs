//! Silicon Labs EFM32 Gecko and EFR32 Wireless Gecko Memory System Controller (MSC).

use crate::traits::TargetMemAccess;
use crate::types::{TargetDescriptor, TargetError};

const MSC_WRITECTRL: u32 = 0x008;
const MSC_WRITECMD: u32 = 0x00C;
const MSC_ADDRB: u32 = 0x010;
const MSC_WDATA: u32 = 0x018;
const MSC_STATUS: u32 = 0x01C;

fn msc_wait_busy<M: TargetMemAccess>(mem: &mut M, base: u32) -> Result<(), TargetError> {
    for _ in 0..100_000 {
        if let Ok(stat) = mem.read32(base + MSC_STATUS) {
            if (stat & 0x01) == 0 { // BUSY bit 0 is clear
                return Ok(());
            }
        }
    }
    Err(TargetError::Timeout)
}

pub fn msc_erase_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    page_addr: u32,
) -> Result<(), TargetError> {
    msc_wait_busy(mem, desc.reg_base)?;

    // Enable write: WREN = 1
    mem.write32(desc.reg_base + MSC_WRITECTRL, 0x01).map_err(|_| TargetError::BusError)?;

    // Set page address in ADDRB
    mem.write32(desc.reg_base + MSC_ADDRB, page_addr).map_err(|_| TargetError::BusError)?;

    // Issue ERASEPAGE command (bit 1)
    mem.write32(desc.reg_base + MSC_WRITECMD, 0x02).map_err(|_| TargetError::BusError)?;

    msc_wait_busy(mem, desc.reg_base)?;

    // Disable write
    mem.write32(desc.reg_base + MSC_WRITECTRL, 0x00).map_err(|_| TargetError::BusError)?;
    Ok(())
}

pub fn msc_program_page<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
    addr: u32,
    data: &[u8],
) -> Result<(), TargetError> {
    msc_wait_busy(mem, desc.reg_base)?;
    mem.write32(desc.reg_base + MSC_WRITECTRL, 0x01).map_err(|_| TargetError::BusError)?;

    let mut offset = 0usize;
    while offset < data.len() {
        let mut word = [0xFFu8; 4];
        let len = (data.len() - offset).min(4);
        word[..len].copy_from_slice(&data[offset..offset + len]);

        let curr_addr = addr + offset as u32;
        mem.write32(desc.reg_base + MSC_ADDRB, curr_addr).map_err(|_| TargetError::BusError)?;
        mem.write32(desc.reg_base + MSC_WDATA, u32::from_le_bytes(word)).map_err(|_| TargetError::BusError)?;

        // WRITEONCE command (bit 3)
        mem.write32(desc.reg_base + MSC_WRITECMD, 0x08).map_err(|_| TargetError::BusError)?;
        msc_wait_busy(mem, desc.reg_base)?;
        offset += 4;
    }

    mem.write32(desc.reg_base + MSC_WRITECTRL, 0x00).map_err(|_| TargetError::BusError)?;
    Ok(())
}
