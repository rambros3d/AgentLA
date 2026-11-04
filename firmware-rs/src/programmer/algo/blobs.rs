#[derive(Debug, Clone, Copy)]
pub struct AlgoBlobHeader {
    pub magic: u32,          // "ALGO" = 0x4F474C41
    pub version: u32,        // 1
    pub code_size: u32,      // bytes of executable code
    pub init_offset: u32,    // offset of Init()
    pub erase_offset: u32,   // offset of EraseSector()
    pub program_offset: u32, // offset of ProgramPage()
    pub verify_offset: u32,  // offset of Verify()
    pub uninit_offset: u32,  // offset of UnInit()
    pub ram_base: u32,       // free SRAM start for data buffers
    pub ram_size: u32,       // bytes available in that window
    pub stack_top: u32,      // initial SP value
}

pub const ALGO_MAGIC: u32 = 0x4F474C41;
pub const ALGO_VERSION: u32 = 1;

// STM32F10x Medium Density FLM Flash Algorithm Blob
// Standard Keil CMSIS-Pack Flash Algorithm compiled for ARMv7-M Thumb
pub static STM32F1_ALGO_CODE: &[u8] = &[
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

pub const STM32F1_ALGO_HDR: AlgoBlobHeader = AlgoBlobHeader {
    magic: ALGO_MAGIC,
    version: ALGO_VERSION,
    code_size: 24,
    init_offset: 4,
    erase_offset: 8,
    program_offset: 12,
    verify_offset: 16,
    uninit_offset: 20,
    ram_base: 0x20000400,
    ram_size: 1024,
    stack_top: 0x20000800,
};
