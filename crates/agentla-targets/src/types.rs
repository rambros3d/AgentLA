//! Core types, errors, descriptors, and silicon families.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectEngineKind {
    Stm32Classic16,
    Stm32Sector,
    Stm32Modern64,
    Stm32DualBankH7,
    PuyaPy32,
    NordicNvmc,
    MicrochipSamd,
    MicrochipSame,
    NxpLpc,
    NxpKinetis,
    TiMspm0,
    SiliconLabsMsc,
    RenesasRa,
    RaspberryPiRom,
    QingKeV2,
    QingKeV4,
    WchBle,
    EspressifRiscvSpi,
    GigaDeviceRiscv,
    HardwareSecurityRescue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiliconFamily {
    Stm32F1,
    Stm32F2,
    Stm32F3,
    Stm32F4,
    Stm32F7,
    Stm32G0,
    Stm32G4,
    Stm32L0,
    Stm32L1,
    Stm32L4,
    Stm32L5,
    Stm32C0,
    Stm32U5,
    Stm32H5,
    Stm32H7,
    Stm32Wb,
    Stm32Wl,
    PuyaPy32F0,
    NordicNrf51,
    NordicNrf52,
    NordicNrf53,
    NordicNrf91,
    MicrochipSamd,
    MicrochipSame,
    NxpLpc800,
    NxpLpc1100,
    NxpLpc54000,
    NxpLpc55S,
    NxpKinetis,
    TiMspm0,
    TiSimpleLink,
    SiliconLabsEfm32,
    RenesasRa2,
    RenesasRa4,
    RenesasRa6,
    RaspberryPiRp2040,
    RaspberryPiRp2350,
    WchQingKeV2,
    WchQingKeV4,
    WchBleSeries,
    EspressifEsp32Riscv,
    GigaDeviceRiscv,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetProtection {
    None,
    ReadoutProtectLevel1,
    ReadoutProtectLevel2,
    ApprotectLocked,
    SecurityBitSet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetError {
    BusError,
    Timeout,
    UnlockFailed,
    VerifyMismatch { addr: u32, expected: u8, actual: u8 },
    UnsupportedTarget,
    TargetProtected(TargetProtection),
    FlashControllerFault(u32),
    InvalidAddress { addr: u32, limit: u32 },
    AlignmentError,
    RescueFailed,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetDescriptor {
    pub name: &'static str,
    pub dev_id: u16,
    pub dev_mask: u16,
    pub family: SiliconFamily,
    pub engine: DirectEngineKind,
    pub flash_base: u32,
    pub flash_size: u32,
    pub ram_base: u32,
    pub ram_size: u32,
    pub page_size: u16,
    pub sector_size: u32,
    pub reg_base: u32,
    pub keys: (u32, u32),
}

impl TargetDescriptor {
    pub const fn unknown() -> Self {
        Self {
            name: "Unknown Target",
            dev_id: 0,
            dev_mask: 0,
            family: SiliconFamily::Unknown,
            engine: DirectEngineKind::Stm32Classic16,
            flash_base: 0x08000000,
            flash_size: 0,
            ram_base: 0x20000000,
            ram_size: 0,
            page_size: 256,
            sector_size: 1024,
            reg_base: 0,
            keys: (0, 0),
        }
    }

    pub fn matches_dev_id(&self, queried_id: u16) -> bool {
        (queried_id & self.dev_mask) == (self.dev_id & self.dev_mask)
    }
}
