//! Direct parameterized flash programming and rescue engines.

pub mod microchip;
pub mod nordic;
pub mod nxp;
pub mod puya;
pub mod raspberrypi;
pub mod renesas;
pub mod rescue;
pub mod riscv;
pub mod silabs;
pub mod stm32;
pub mod ti;

use crate::traits::TargetMemAccess;
use crate::types::{DirectEngineKind, TargetDescriptor, TargetError};

pub struct FlashEngineRunner;

impl FlashEngineRunner {
    /// Erase a specific flash sector for the given target descriptor.
    pub fn erase_sector<M: TargetMemAccess>(
        mem: &mut M,
        desc: &TargetDescriptor,
        sector_addr: u32,
    ) -> Result<(), TargetError> {
        match desc.engine {
            DirectEngineKind::Stm32Classic16 => stm32::classic16_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::Stm32Sector => stm32::sector_erase(mem, desc, sector_addr),
            DirectEngineKind::Stm32Modern64 => stm32::modern64_erase_page(mem, desc, sector_addr),
            DirectEngineKind::Stm32DualBankH7 => stm32::h7_dualbank_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::PuyaPy32 => puya::py32_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::NordicNvmc => nordic::nvmc_erase_page(mem, desc, sector_addr),
            DirectEngineKind::MicrochipSamd => microchip::samd_erase_row(mem, desc, sector_addr),
            DirectEngineKind::MicrochipSame => microchip::same_erase_block(mem, desc, sector_addr),
            DirectEngineKind::NxpLpc => nxp::lpc_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::NxpKinetis => nxp::kinetis_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::TiMspm0 => ti::mspm0_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::SiliconLabsMsc => silabs::msc_erase_page(mem, desc, sector_addr),
            DirectEngineKind::RenesasRa => renesas::ra_erase_block(mem, desc, sector_addr),
            DirectEngineKind::RaspberryPiRom => raspberrypi::rp_rom_erase(mem, desc, sector_addr, desc.sector_size),
            DirectEngineKind::QingKeV2 => riscv::qingke_v2_erase_page(mem, desc, sector_addr),
            DirectEngineKind::QingKeV4 => riscv::qingke_v4_erase_page(mem, desc, sector_addr),
            DirectEngineKind::WchBle => riscv::wch_ble_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::EspressifRiscvSpi => riscv::esp32_spi_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::GigaDeviceRiscv => riscv::gd32v_erase_sector(mem, desc, sector_addr),
            DirectEngineKind::HardwareSecurityRescue => Err(TargetError::UnsupportedTarget),
        }
    }

    /// Program a single page/buffer of bytes into the target's flash memory.
    pub fn program_page<M: TargetMemAccess>(
        mem: &mut M,
        desc: &TargetDescriptor,
        addr: u32,
        data: &[u8],
    ) -> Result<(), TargetError> {
        match desc.engine {
            DirectEngineKind::Stm32Classic16 => stm32::classic16_program_page(mem, desc, addr, data),
            DirectEngineKind::Stm32Sector => stm32::sector_program_page(mem, desc, addr, data),
            DirectEngineKind::Stm32Modern64 => stm32::modern64_program_page(mem, desc, addr, data),
            DirectEngineKind::Stm32DualBankH7 => stm32::h7_dualbank_program_page(mem, desc, addr, data),
            DirectEngineKind::PuyaPy32 => puya::py32_program_page(mem, desc, addr, data),
            DirectEngineKind::NordicNvmc => nordic::nvmc_program_page(mem, desc, addr, data),
            DirectEngineKind::MicrochipSamd => microchip::samd_program_page(mem, desc, addr, data),
            DirectEngineKind::MicrochipSame => microchip::same_program_page(mem, desc, addr, data),
            DirectEngineKind::NxpLpc => nxp::lpc_program_page(mem, desc, addr, data),
            DirectEngineKind::NxpKinetis => nxp::kinetis_program_page(mem, desc, addr, data),
            DirectEngineKind::TiMspm0 => ti::mspm0_program_page(mem, desc, addr, data),
            DirectEngineKind::SiliconLabsMsc => silabs::msc_program_page(mem, desc, addr, data),
            DirectEngineKind::RenesasRa => renesas::ra_program_page(mem, desc, addr, data),
            DirectEngineKind::RaspberryPiRom => raspberrypi::rp_rom_program(mem, desc, addr, data),
            DirectEngineKind::QingKeV2 => riscv::qingke_v2_program_page(mem, desc, addr, data),
            DirectEngineKind::QingKeV4 => riscv::qingke_v4_program_page(mem, desc, addr, data),
            DirectEngineKind::WchBle => riscv::wch_ble_program_page(mem, desc, addr, data),
            DirectEngineKind::EspressifRiscvSpi => riscv::esp32_spi_program_page(mem, desc, addr, data),
            DirectEngineKind::GigaDeviceRiscv => riscv::gd32v_program_page(mem, desc, addr, data),
            DirectEngineKind::HardwareSecurityRescue => Err(TargetError::UnsupportedTarget),
        }
    }

    /// Readback and verify programmed flash memory content against expected data.
    pub fn verify<M: TargetMemAccess>(
        mem: &mut M,
        _desc: &TargetDescriptor,
        addr: u32,
        data: &[u8],
    ) -> Result<(), TargetError> {
        let mut chunk = [0u8; 64];
        let mut offset = 0usize;
        while offset < data.len() {
            let len = (data.len() - offset).min(chunk.len());
            mem.read_block(addr + offset as u32, &mut chunk[..len])
                .map_err(|_| TargetError::BusError)?;
            for i in 0..len {
                if chunk[i] != data[offset + i] {
                    return Err(TargetError::VerifyMismatch {
                        addr: addr + (offset + i) as u32,
                        expected: data[offset + i],
                        actual: chunk[i],
                    });
                }
            }
            offset += len;
        }
        Ok(())
    }

    /// Unbrick or mass-erase protected / locked microcontroller.
    pub fn unbrick<M: TargetMemAccess>(
        mem: &mut M,
        desc: &TargetDescriptor,
    ) -> Result<(), TargetError> {
        rescue::execute_unbrick(mem, desc)
    }
}
