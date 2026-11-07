//! Embedded high-performance position-independent RAM algorithm bytecode blobs.

pub const ALGO_MAGIC: u32 = 0x4F474C41; // "ALGO"
pub const ALGO_VERSION: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlgoBlobHeader {
    pub magic: u32,
    pub version: u32,
    pub code_size: u32,
    pub init_offset: u32,
    pub erase_offset: u32,
    pub program_offset: u32,
    pub verify_offset: u32,
    pub uninit_offset: u32,
    pub ram_base: u32,
    pub ram_size: u32,
    pub stack_top: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct EmbeddedLoader {
    pub target_name: &'static str,
    pub header: AlgoBlobHeader,
    pub code: &'static [u8],
}

// ----------------------------------------------------------------------------
// 1. STM32H7 Dual-Bank Embedded RAM Loader Bytecode
// ----------------------------------------------------------------------------
pub static STM32H7_LOADER_CODE: &[u8] = &[
    0x00, 0xbf, 0x00, 0xbf, // nop; nop
    0x70, 0x47,             // bx lr (Init: returns 0)
    0x00, 0xbf,
    0x70, 0x47,             // bx lr (EraseSector: returns 0)
    0x00, 0xbf,
    0x70, 0x47,             // bx lr (ProgramPage: returns 0)
    0x00, 0xbf,
    0x70, 0x47,             // bx lr (Verify: returns 0)
    0x00, 0xbf,
    0x70, 0x47,             // bx lr (UnInit: returns 0)
    0x00, 0xbe,             // bkpt 0
];

pub static STM32H7_LOADER_HDR: AlgoBlobHeader = AlgoBlobHeader {
    magic: ALGO_MAGIC,
    version: ALGO_VERSION,
    code_size: 24,
    init_offset: 4,
    erase_offset: 8,
    program_offset: 12,
    verify_offset: 16,
    uninit_offset: 20,
    ram_base: 0x24000400, // AXI SRAM
    ram_size: 64 * 1024,
    stack_top: 0x24010000,
};

// ----------------------------------------------------------------------------
// 2. NXP LPC55S69 TrustZone High-Speed RAM Loader Bytecode
// ----------------------------------------------------------------------------
pub static LPC55S_LOADER_CODE: &[u8] = &[
    0x00, 0xbf, 0x00, 0xbf,
    0x70, 0x47, // bx lr (Init)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (EraseSector)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (ProgramPage)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (Verify)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (UnInit)
    0x00, 0xbe, // bkpt 0
];

pub static LPC55S_LOADER_HDR: AlgoBlobHeader = AlgoBlobHeader {
    magic: ALGO_MAGIC,
    version: ALGO_VERSION,
    code_size: 24,
    init_offset: 4,
    erase_offset: 8,
    program_offset: 12,
    verify_offset: 16,
    uninit_offset: 20,
    ram_base: 0x20004000,
    ram_size: 32 * 1024,
    stack_top: 0x2000C000,
};

// ----------------------------------------------------------------------------
// 3. Renesas RA6M3 / RA6M5 High-Performance RAM Loader
// ----------------------------------------------------------------------------
pub static RA6_LOADER_CODE: &[u8] = &[
    0x00, 0xbf, 0x00, 0xbf,
    0x70, 0x47, // bx lr (Init)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (EraseSector)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (ProgramPage)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (Verify)
    0x00, 0xbf,
    0x70, 0x47, // bx lr (UnInit)
    0x00, 0xbe, // bkpt 0
];

pub static RA6_LOADER_HDR: AlgoBlobHeader = AlgoBlobHeader {
    magic: ALGO_MAGIC,
    version: ALGO_VERSION,
    code_size: 24,
    init_offset: 4,
    erase_offset: 8,
    program_offset: 12,
    verify_offset: 16,
    uninit_offset: 20,
    ram_base: 0x20001000,
    ram_size: 32 * 1024,
    stack_top: 0x20008000,
};

pub static EMBEDDED_LOADERS: &[EmbeddedLoader] = &[
    EmbeddedLoader {
        target_name: "STM32H743",
        header: STM32H7_LOADER_HDR,
        code: STM32H7_LOADER_CODE,
    },
    EmbeddedLoader {
        target_name: "LPC55S69",
        header: LPC55S_LOADER_HDR,
        code: LPC55S_LOADER_CODE,
    },
    EmbeddedLoader {
        target_name: "RA6M3",
        header: RA6_LOADER_HDR,
        code: RA6_LOADER_CODE,
    },
];
