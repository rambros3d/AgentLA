//! Multi-vendor hardware security and unbricking engine.

use crate::traits::TargetMemAccess;
use crate::types::{SiliconFamily, TargetDescriptor, TargetError};

pub fn execute_unbrick<M: TargetMemAccess>(
    mem: &mut M,
    desc: &TargetDescriptor,
) -> Result<(), TargetError> {
    match desc.family {
        SiliconFamily::Stm32F1 | SiliconFamily::Stm32F3 | SiliconFamily::Stm32G0 | SiliconFamily::Stm32G4 => {
            stm32_rdp_regress(mem, desc)
        }
        SiliconFamily::NordicNrf51 | SiliconFamily::NordicNrf52 | SiliconFamily::NordicNrf53 | SiliconFamily::NordicNrf91 => {
            nordic_ctrl_ap_erase(mem)
        }
        SiliconFamily::NxpKinetis | SiliconFamily::NxpLpc55S => {
            nxp_mdm_ap_mass_erase(mem)
        }
        SiliconFamily::MicrochipSamd | SiliconFamily::MicrochipSame => {
            samd_dsu_chip_erase(mem)
        }
        _ => Err(TargetError::UnsupportedTarget),
    }
}

/// STM32 RDP Level 1 -> Level 0 regress (triggers hardware mass erase)
fn stm32_rdp_regress<M: TargetMemAccess>(mem: &mut M, desc: &TargetDescriptor) -> Result<(), TargetError> {
    let optkeyr = desc.reg_base + 0x08;
    let cr_addr = desc.reg_base + 0x10;

    // Unlock option bytes
    let _ = mem.write32(optkeyr, 0x45670123);
    let _ = mem.write32(optkeyr, 0xCDEF89AB);

    // Erase option bytes (OPTER bit 5)
    mem.write32(cr_addr, 0x20).map_err(|_| TargetError::BusError)?;
    mem.write32(cr_addr, 0x20 | 0x40).map_err(|_| TargetError::BusError)?; // STRT

    // Wait for BSY clear
    for _ in 0..100_000 {
        if let Ok(sr) = mem.read32(desc.reg_base + 0x0C) {
            if (sr & 0x01) == 0 {
                break;
            }
        }
    }

    // Program RDP Key 0x5AA5 into option bytes
    mem.write32(cr_addr, 0x10).map_err(|_| TargetError::BusError)?; // OPTPG bit 4
    let _ = mem.write32(0x1FFFF800, 0x00FF5AA5);

    for _ in 0..100_000 {
        if let Ok(sr) = mem.read32(desc.reg_base + 0x0C) {
            if (sr & 0x01) == 0 {
                mem.write32(cr_addr, 0x00).map_err(|_| TargetError::BusError)?;
                return Ok(());
            }
        }
    }
    Err(TargetError::RescueFailed)
}

/// Nordic CTRL-AP Mass Erase
fn nordic_ctrl_ap_erase<M: TargetMemAccess>(mem: &mut M) -> Result<(), TargetError> {
    // Assert hardware reset
    let _ = mem.assert_reset(true);
    // Write ERASEALL in CTRL-AP (mapped at AP #1 in CoreSight or direct register)
    let _ = mem.write32(0x4001E50C, 0x01); // NVMC ERASEALL
    let _ = mem.assert_reset(false);
    Ok(())
}

/// NXP MDM-AP Mass Erase
fn nxp_mdm_ap_mass_erase<M: TargetMemAccess>(mem: &mut M) -> Result<(), TargetError> {
    let _ = mem.assert_reset(true);
    // MDM-AP Control Register bit 0: Flash Mass Erase Request
    let _ = mem.write32(0x01000004, 0x01);
    let _ = mem.assert_reset(false);
    Ok(())
}

/// Microchip SAMD/SAME DSU Chip Erase
fn samd_dsu_chip_erase<M: TargetMemAccess>(mem: &mut M) -> Result<(), TargetError> {
    // SAMD DSU CTRL at 0x41002000: Write 0x00000010 (Chip Erase bit 4)
    mem.write32(0x41002000, 0x10).map_err(|_| TargetError::BusError)?;
    for _ in 0..50_000 {
        if let Ok(stat) = mem.read32(0x41002001) { // DSU STATUSA
            if (stat & 0x01) != 0 { // Done
                return Ok(());
            }
        }
    }
    Ok(())
}
