#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetFamily {
    Stm32F1,
    Stm32F4,
    Stm32H5,
    Rp2040,
    Rp2350,
    Ch32V003,
    Ch32V203,
    Py32F0,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
pub struct TargetInfo {
    pub name: &'static str,
    pub family: TargetFamily,
    pub idcode: u32,
    pub flash_base: u32,
    pub flash_size: u32,
    pub ram_base: u32,
    pub ram_size: u32,
    pub sector_size: u32,
    pub page_size: u32,
}

impl TargetInfo {
    pub const fn unknown() -> Self {
        Self {
            name: "Unknown Target",
            family: TargetFamily::Unknown,
            idcode: 0,
            flash_base: 0x08000000,
            flash_size: 0,
            ram_base: 0x20000000,
            ram_size: 0,
            sector_size: 1024,
            page_size: 256,
        }
    }
}

pub fn identify_cortex_target(dpidr: u32, dbgmcu_id: u32, sysinfo_id: u32) -> TargetInfo {
    // 1. Check RP2040 / RP2350 sysinfo
    if (sysinfo_id >> 16) == 0x2040 || sysinfo_id == 0x00002040 {
        return TargetInfo {
            name: "Raspberry Pi RP2040 (Dual Cortex-M0+)",
            family: TargetFamily::Rp2040,
            idcode: sysinfo_id,
            flash_base: 0x10000000,
            flash_size: 2 * 1024 * 1024,
            ram_base: 0x20000000,
            ram_size: 264 * 1024,
            sector_size: 4096,
            page_size: 256,
        };
    }
    if (sysinfo_id >> 16) == 0x2350 || sysinfo_id == 0x00002350 {
        return TargetInfo {
            name: "Raspberry Pi RP2350 (Dual Cortex-M33)",
            family: TargetFamily::Rp2350,
            idcode: sysinfo_id,
            flash_base: 0x10000000,
            flash_size: 4 * 1024 * 1024,
            ram_base: 0x20000000,
            ram_size: 520 * 1024,
            sector_size: 4096,
            page_size: 256,
        };
    }

    // 2. Check STM32 DBGMCU IDCODE
    let dev_id = dbgmcu_id & 0x0FFF;
    match dev_id {
        0x410 => TargetInfo {
            name: "STM32F103 Medium Density",
            family: TargetFamily::Stm32F1,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 128 * 1024,
            ram_base: 0x20000000,
            ram_size: 20 * 1024,
            sector_size: 1024,
            page_size: 1024,
        },
        0x412 => TargetInfo {
            name: "STM32F103 Low Density",
            family: TargetFamily::Stm32F1,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 32 * 1024,
            ram_base: 0x20000000,
            ram_size: 10 * 1024,
            sector_size: 1024,
            page_size: 1024,
        },
        0x414 => TargetInfo {
            name: "STM32F103 High Density",
            family: TargetFamily::Stm32F1,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 512 * 1024,
            ram_base: 0x20000000,
            ram_size: 64 * 1024,
            sector_size: 2048,
            page_size: 2048,
        },
        0x431 => TargetInfo {
            name: "STM32F411 BlackPill",
            family: TargetFamily::Stm32F4,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 512 * 1024,
            ram_base: 0x20000000,
            ram_size: 128 * 1024,
            sector_size: 16 * 1024,
            page_size: 256,
        },
        0x484 => TargetInfo {
            name: "STM32H5 High Performance Cortex-M33",
            family: TargetFamily::Stm32H5,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 2048 * 1024,
            ram_base: 0x20000000,
            ram_size: 640 * 1024,
            sector_size: 8192,
            page_size: 256,
        },
        // Puya PY32
        0x440 | 0x444 if (dpidr & 0x0FFF0000) == 0x0BC10000 => TargetInfo {
            name: "Puya PY32F0 Ultra-Low-Power Cortex-M0+",
            family: TargetFamily::Py32F0,
            idcode: dbgmcu_id,
            flash_base: 0x08000000,
            flash_size: 64 * 1024,
            ram_base: 0x20000000,
            ram_size: 4 * 1024,
            sector_size: 128,
            page_size: 128,
        },
        _ => {
            if dpidr == 0x1BA01477 || dpidr == 0x2BA01477 {
                TargetInfo {
                    name: "STM32F103 BluePill (ARM Cortex-M3)",
                    family: TargetFamily::Stm32F1,
                    idcode: if dbgmcu_id != 0 { dbgmcu_id } else { dpidr },
                    flash_base: 0x08000000,
                    flash_size: 64 * 1024,
                    ram_base: 0x20000000,
                    ram_size: 20 * 1024,
                    sector_size: 1024,
                    page_size: 1024,
                }
            } else if dbgmcu_id == 0x20000000 || (dpidr == 0x0BC11477 && dbgmcu_id == 0) {
                TargetInfo {
                    name: "Puya PY32F002A / PY32F030",
                    family: TargetFamily::Py32F0,
                    idcode: if dbgmcu_id != 0 { dbgmcu_id } else { dpidr },
                    flash_base: 0x08000000,
                    flash_size: 32 * 1024,
                    ram_base: 0x20000000,
                    ram_size: 4 * 1024,
                    sector_size: 128,
                    page_size: 128,
                }
            } else {
                TargetInfo {
                    name: "Generic ARM Cortex-M Target",
                    family: TargetFamily::Unknown,
                    idcode: if dbgmcu_id != 0 { dbgmcu_id } else { dpidr },
                    flash_base: 0x08000000,
                    flash_size: 64 * 1024,
                    ram_base: 0x20000000,
                    ram_size: 16 * 1024,
                    sector_size: 1024,
                    page_size: 256,
                }
            }
        }
    }
}

pub fn ch32v003_info(part_id: u32) -> TargetInfo {
    TargetInfo {
        name: "WCH CH32V003 (QingKe V2 RISC-V 1-Wire)",
        family: TargetFamily::Ch32V003,
        idcode: part_id,
        flash_base: 0x08000000,
        flash_size: 16 * 1024,
        ram_base: 0x20000000,
        ram_size: 2 * 1024,
        sector_size: 64,
        page_size: 64,
    }
}

pub fn ch32v203_info(status: u32) -> TargetInfo {
    TargetInfo {
        name: "WCH CH32V203 / CH592 (QingKe V4 RISC-V 2-Wire)",
        family: TargetFamily::Ch32V203,
        idcode: status,
        flash_base: 0x08000000,
        flash_size: 64 * 1024,
        ram_base: 0x20000000,
        ram_size: 20 * 1024,
        sector_size: 256,
        page_size: 256,
    }
}
