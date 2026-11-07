//! Raw PIO assembly instruction definitions and offset constants.

// ============================================================================
// 1. ARM SWD (swd_probe.pio)
// ============================================================================
pub const SWD_OFFSET_WRITE_CMD: u8 = 0;
pub const SWD_OFFSET_TURNAROUND_CMD: u8 = 0;
pub const SWD_OFFSET_GET_NEXT_CMD: u8 = 3;
pub const SWD_OFFSET_READ_CMD: u8 = 8;

pub const SWD_INSTRUCTIONS: [u16; 11] = [
    0x80a0, //  0: pull   block
    0x7101, //  1: out    pins, 1         side 0 [1]
    0x1941, //  2: jmp    x--, 1          side 1 [1]
            //     .wrap_target
    0x90a0, //  3: pull   block           side 0
    0x6028, //  4: out    x, 8
    0x6081, //  5: out    pindirs, 1
    0x60a5, //  6: out    pc, 5
    0xa042, //  7: nop
    0x5901, //  8: in     pins, 1         side 1 [1]
    0x1047, //  9: jmp    x--, 7          side 0
    0x8020, // 10: push   block
            //     .wrap
];

pub const SWD_WRAP_SOURCE: u8 = 10;
pub const SWD_WRAP_TARGET: u8 = 3;

// ============================================================================
// 2. WCH 1-Wire SWIO (Push-Pull Tx, Active Pull-Up Rx)
// ============================================================================
pub const SWIO_INSTRUCTIONS: [u16; 24] = [
    0xf281, //  0: set pindirs, 1       side 1 [2]
    0x90a0, //  1: pull   block         side 1 [0]
    0x6158, //  2: out    y, 24         side 0 [1]
    0xb242, //  3: nop                  side 1 [2]
    0x6021, //  4: out    x, 1          side 0 [0]
    0x0027, //  5: jmp    !x, 7         side 0 [0]
    0xa542, //  6: nop                  side 0 [5]
    0x12e4, //  7: jmp    !osre, 4      side 1 [2]
    0x1031, //  8: jmp    !x, 17        side 1 [0]
    0xf03f, //  9: set    x, 31         side 1 [0]
    0xe181, // 10: set    pindirs, 1    side 0 [1]
    0xb042, // 11: nop                  side 1 [0]
    0xf180, // 12: set    pindirs, 0    side 1 [1]
    0x5201, // 13: in     pins, 1       side 1 [2]
    0x124a, // 14: jmp    x--, 10       side 1 [2]
    0xf681, // 15: set    pindirs, 1    side 1 [6]
    0x1a00, // 16: jmp    0             side 1 [10]
    0x91a0, // 17: pull   block         side 1 [1]
    0x6021, // 18: out    x, 1          side 0 [0]
    0x0035, // 19: jmp    !x, 21        side 0 [0]
    0xa542, // 20: nop                  side 0 [5]
    0x12f2, // 21: jmp    !osre, 18     side 1 [2]
    0xb642, // 22: nop                  side 1 [6]
    0x1a00, // 23: jmp    0             side 1 [10]
];

pub const SWIO_WRAP_SOURCE: u8 = 23;
pub const SWIO_WRAP_TARGET: u8 = 0;

// ============================================================================
// 3. WCH 2-Wire RVSWD (rvswd.pio)
// ============================================================================
pub const RVSWD_OFFSET_GET_NEXT_CMD: u8 = 0;
pub const RVSWD_OFFSET_WRITE_BITS: u8 = 4;
pub const RVSWD_OFFSET_READ_BITS: u8 = 10;
pub const RVSWD_OFFSET_START: u8 = 15;
pub const RVSWD_OFFSET_STOP: u8 = 19;
pub const RVSWD_OFFSET_RESET: u8 = 23;

pub const RVSWD_INSTRUCTIONS: [u16; 29] = [
    0x98a0, //  0: pull   block           side 1
    0x6028, //  1: out    x, 8
    0x6081, //  2: out    pindirs, 1
    0x60a5, //  3: out    pc, 5
    0x80a0, //  4: pull   block
    0x7201, //  5: out    pins, 1         side 0 [2]
    0xba42, //  6: nop                    side 1 [2]
    0x0045, //  7: jmp    x--, 5
    0x0000, //  8: jmp    0
    0xb842, //  9: nop                    side 1
    0xb242, // 10: nop                    side 0 [2]
    0x5a01, // 11: in     pins, 1         side 1 [2]
    0x004a, // 12: jmp    x--, 10
    0x8020, // 13: push   block
    0x0000, // 14: jmp    0
    0xe081, // 15: set    pindirs, 1
    0xfa01, // 16: set    pins, 1         side 1 [2]
    0xfa00, // 17: set    pins, 0         side 1 [2]
    0x0000, // 18: jmp    0
    0xfa00, // 19: set    pins, 0         side 1 [2]
    0xfa01, // 20: set    pins, 1         side 1 [2]
    0xe080, // 21: set    pindirs, 0
    0x0000, // 22: jmp    0
    0xe081, // 23: set    pindirs, 1
    0xfa01, // 24: set    pins, 1         side 1 [2]
    0xb242, // 25: nop                    side 0 [2]
    0xba42, // 26: nop                    side 1 [2]
    0x0059, // 27: jmp    x--, 25
    0x0013, // 28: jmp    19
];

pub const RVSWD_WRAP_SOURCE: u8 = 28;
pub const RVSWD_WRAP_TARGET: u8 = 0;
