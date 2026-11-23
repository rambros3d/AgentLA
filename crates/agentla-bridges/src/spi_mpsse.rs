//! FTDI MPSSE (Multi-Protocol Synchronous Serial Engine) protocol engine.
//!
//! Provides a `#![no_std]` byte-stream parser and state machine conforming to
//! FTDI FT2232H / FT232H MPSSE specifications. Allows standard host tools
//! (`flashrom`, `pyftdi`, `openocd`, `sigrok`) to drive hardware SPI over
//! USB Bulk IN/OUT endpoints.

#![allow(dead_code)]

pub const MPSSE_CMD_BAD: u8 = 0xFA;

// Write commands (Data out)
pub const CMD_DATA_OUT_BYTES_NEG: u8 = 0x10; // Write bytes, negative clock edge, MSB first
pub const CMD_DATA_OUT_BYTES_POS: u8 = 0x11; // Write bytes, positive clock edge, MSB first
pub const CMD_DATA_OUT_BITS_NEG: u8 = 0x12;  // Write bits, negative clock edge, MSB first
pub const CMD_DATA_OUT_BITS_POS: u8 = 0x13;  // Write bits, positive clock edge, MSB first
pub const CMD_DATA_OUT_BYTES_LSB_NEG: u8 = 0x18; // Write bytes, negative clock edge, LSB first
pub const CMD_DATA_OUT_BYTES_LSB_POS: u8 = 0x19; // Write bytes, positive clock edge, LSB first
pub const CMD_DATA_OUT_BITS_LSB_NEG: u8 = 0x1A;  // Write bits, negative clock edge, LSB first
pub const CMD_DATA_OUT_BITS_LSB_POS: u8 = 0x1B;  // Write bits, positive clock edge, LSB first

// Read commands (Data in)
pub const CMD_DATA_IN_BYTES_POS: u8 = 0x20;  // Read bytes, positive clock edge, MSB first
pub const CMD_DATA_IN_BYTES_NEG: u8 = 0x24;  // Read bytes, negative clock edge, MSB first
pub const CMD_DATA_IN_BITS_POS: u8 = 0x22;   // Read bits, positive clock edge, MSB first
pub const CMD_DATA_IN_BITS_NEG: u8 = 0x26;   // Read bits, negative clock edge, MSB first
pub const CMD_DATA_IN_BYTES_LSB_POS: u8 = 0x28; // Read bytes, positive clock edge, LSB first
pub const CMD_DATA_IN_BYTES_LSB_NEG: u8 = 0x2C; // Read bytes, negative clock edge, LSB first
pub const CMD_DATA_IN_BITS_LSB_POS: u8 = 0x2A;  // Read bits, positive clock edge, LSB first
pub const CMD_DATA_IN_BITS_LSB_NEG: u8 = 0x2E;  // Read bits, negative clock edge, LSB first

// Full-duplex commands (Data in & out)
pub const CMD_DATA_IN_OUT_BYTES_POS_NEG: u8 = 0x31; // Out +ve edge, In -ve edge, MSB
pub const CMD_DATA_IN_OUT_BYTES_NEG_POS: u8 = 0x34; // Out -ve edge, In +ve edge, MSB
pub const CMD_DATA_IN_OUT_BYTES_POS_POS: u8 = 0x35; // Out +ve edge, In +ve edge, MSB
pub const CMD_DATA_IN_OUT_BYTES_NEG_NEG: u8 = 0x36; // Out -ve edge, In -ve edge, MSB
pub const CMD_DATA_IN_OUT_BITS_POS_NEG: u8 = 0x33;
pub const CMD_DATA_IN_OUT_BITS_NEG_POS: u8 = 0x36;

pub const CMD_DATA_IN_OUT_BYTES_LSB_POS_NEG: u8 = 0x39;
pub const CMD_DATA_IN_OUT_BYTES_LSB_NEG_POS: u8 = 0x3C;

// Pin / GPIO Control Commands
pub const CMD_SET_BITS_LOW: u8 = 0x80;  // Set data bits low byte (TCK, TDI, TDO, TMS, GPIOL0..3)
pub const CMD_READ_BITS_LOW: u8 = 0x81; // Read data bits low byte
pub const CMD_SET_BITS_HIGH: u8 = 0x82; // Set data bits high byte (GPIOH0..7)
pub const CMD_READ_BITS_HIGH: u8 = 0x83;// Read data bits high byte

// Loopback
pub const CMD_LOOPBACK_ON: u8 = 0x84;
pub const CMD_LOOPBACK_OFF: u8 = 0x85;

// Clock control
pub const CMD_SET_TCK_DIVISOR: u8 = 0x86; // 2 data bytes: Low, High
pub const CMD_DIS_DIV_BY_5: u8 = 0x8A;    // Disable divide-by-5 (60 MHz base clock)
pub const CMD_EN_DIV_BY_5: u8 = 0x8B;     // Enable divide-by-5 (12 MHz base clock)
pub const CMD_EN_3PHASE: u8 = 0x8C;       // Enable 3-phase clocking
pub const CMD_DIS_3PHASE: u8 = 0x8D;      // Disable 3-phase clocking
pub const CMD_CLOCK_BITS_NO_DATA: u8 = 0x8E;
pub const CMD_CLOCK_BYTES_NO_DATA: u8 = 0x8F;

// Pin masks for LowByte
pub const PIN_MASK_SK: u8 = 1 << 0;  // Clock (SCK)
pub const PIN_MASK_DO: u8 = 1 << 1;  // Data Out (MOSI)
pub const PIN_MASK_DI: u8 = 1 << 2;  // Data In (MISO)
pub const PIN_MASK_CS: u8 = 1 << 3;  // Chip Select (CS)

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpsseSpiMode {
    Mode0, // CPOL=0, CPHA=0 (Out on -ve, in on +ve)
    Mode1, // CPOL=0, CPHA=1 (Out on +ve, in on -ve)
    Mode2, // CPOL=1, CPHA=0 (Out on +ve, in on -ve)
    Mode3, // CPOL=1, CPHA=1 (Out on -ve, in on +ve)
}

impl MpsseSpiMode {
    pub fn mode_number(&self) -> u8 {
        match self {
            Self::Mode0 => 0,
            Self::Mode1 => 1,
            Self::Mode2 => 2,
            Self::Mode3 => 3,
        }
    }
}

/// Action produced by parsing an MPSSE command stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MpsseAction<'a> {
    /// Full-duplex or half-duplex SPI transfer.
    /// `tx_data`: Slice of bytes to transmit. If empty, transmit 0xFF.
    /// `rx_len`: Number of bytes to receive back from MISO into response buffer.
    /// `mode`: Current active SPI clock mode.
    /// `lsb_first`: Bit order.
    SpiTransfer {
        tx_data: &'a [u8],
        rx_len: usize,
        mode: MpsseSpiMode,
        lsb_first: bool,
    },
    /// Low-byte GPIO pins updated (CS, SCK, MOSI directions and levels).
    GpioSetLow {
        value: u8,
        direction: u8,
    },
    /// High-byte GPIO pins updated.
    GpioSetHigh {
        value: u8,
        direction: u8,
    },
    /// Read low-byte pin state request.
    GpioReadLow,
    /// Read high-byte pin state request.
    GpioReadHigh,
    /// Clock speed divisor updated: frequency in Hz.
    SpeedConfigured(u32),
    /// Invalid opcode echo (0xFA followed by opcode).
    BadCommand(u8),
}

/// FTDI MPSSE Protocol State Machine.
#[derive(Debug, Clone, Copy)]
pub struct MpsseEngine {
    pub low_val: u8,
    pub low_dir: u8,
    pub high_val: u8,
    pub high_dir: u8,
    pub divisor: u16,
    pub div_by_5: bool,
    pub loopback: bool,
    pub three_phase: bool,
    pub default_mode: MpsseSpiMode,
}

impl Default for MpsseEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MpsseEngine {
    pub const fn new() -> Self {
        Self {
            low_val: 0x08, // CS inactive high (bit 3)
            low_dir: 0x0B, // SK, DO, CS output (bits 0, 1, 3); DI input (bit 2)
            high_val: 0x00,
            high_dir: 0x00,
            divisor: 0x0001,
            div_by_5: false,
            loopback: false,
            three_phase: false,
            default_mode: MpsseSpiMode::Mode0,
        }
    }

    /// Calculate active SPI clock frequency in Hz based on divisor and div_by_5 flag.
    pub fn clock_hz(&self) -> u32 {
        let base_clk = if self.div_by_5 { 12_000_000 } else { 60_000_000 };
        base_clk / ((1 + self.divisor as u32) * 2)
    }

    /// Check if Chip Select (bit 3) is asserted (active low = 0).
    pub fn is_cs_asserted(&self) -> bool {
        (self.low_val & PIN_MASK_CS) == 0
    }

    /// Process a stream of MPSSE command bytes.
    ///
    /// For every parsed command, calls `on_action` callback.
    /// Returns the number of bytes consumed from `input`.
    pub fn process_stream<'a, F>(
        &mut self,
        input: &'a [u8],
        mut on_action: F,
    ) -> usize
    where
        F: FnMut(MpsseAction<'a>),
    {
        let mut idx = 0;
        let len = input.len();

        while idx < len {
            let cmd = input[idx];
            idx += 1;

            match cmd {
                // Pin Control
                CMD_SET_BITS_LOW => {
                    if idx + 2 <= len {
                        self.low_val = input[idx];
                        self.low_dir = input[idx + 1];
                        idx += 2;
                        on_action(MpsseAction::GpioSetLow {
                            value: self.low_val,
                            direction: self.low_dir,
                        });
                    } else {
                        return idx - 1; // Wait for complete command
                    }
                }

                CMD_READ_BITS_LOW => {
                    on_action(MpsseAction::GpioReadLow);
                }

                CMD_SET_BITS_HIGH => {
                    if idx + 2 <= len {
                        self.high_val = input[idx];
                        self.high_dir = input[idx + 1];
                        idx += 2;
                        on_action(MpsseAction::GpioSetHigh {
                            value: self.high_val,
                            direction: self.high_dir,
                        });
                    } else {
                        return idx - 1;
                    }
                }

                CMD_READ_BITS_HIGH => {
                    on_action(MpsseAction::GpioReadHigh);
                }

                // Clock Control
                CMD_SET_TCK_DIVISOR => {
                    if idx + 2 <= len {
                        let l = input[idx] as u16;
                        let h = input[idx + 1] as u16;
                        self.divisor = (h << 8) | l;
                        idx += 2;
                        on_action(MpsseAction::SpeedConfigured(self.clock_hz()));
                    } else {
                        return idx - 1;
                    }
                }

                CMD_DIS_DIV_BY_5 => {
                    self.div_by_5 = false;
                    on_action(MpsseAction::SpeedConfigured(self.clock_hz()));
                }

                CMD_EN_DIV_BY_5 => {
                    self.div_by_5 = true;
                    on_action(MpsseAction::SpeedConfigured(self.clock_hz()));
                }

                CMD_EN_3PHASE => {
                    self.three_phase = true;
                }

                CMD_DIS_3PHASE => {
                    self.three_phase = false;
                }

                CMD_LOOPBACK_ON => {
                    self.loopback = true;
                }

                CMD_LOOPBACK_OFF => {
                    self.loopback = false;
                }

                CMD_CLOCK_BITS_NO_DATA => {
                    if idx < len {
                        idx += 1; // Discard bit count
                    } else {
                        return idx - 1;
                    }
                }

                CMD_CLOCK_BYTES_NO_DATA => {
                    if idx + 2 <= len {
                        idx += 2; // Discard byte count
                    } else {
                        return idx - 1;
                    }
                }

                // Data Shifting: Byte Out (Write only)
                CMD_DATA_OUT_BYTES_NEG
                | CMD_DATA_OUT_BYTES_POS
                | CMD_DATA_OUT_BYTES_LSB_NEG
                | CMD_DATA_OUT_BYTES_LSB_POS => {
                    if idx + 2 <= len {
                        let count = ((input[idx + 1] as usize) << 8) | (input[idx] as usize);
                        let xfer_len = count + 1;
                        idx += 2;
                        if idx + xfer_len <= len {
                            let tx_slice = &input[idx..idx + xfer_len];
                            idx += xfer_len;
                            let lsb = (cmd == CMD_DATA_OUT_BYTES_LSB_NEG) || (cmd == CMD_DATA_OUT_BYTES_LSB_POS);
                            let mode = if (cmd & 1) != 0 { MpsseSpiMode::Mode1 } else { MpsseSpiMode::Mode0 };
                            on_action(MpsseAction::SpiTransfer {
                                tx_data: tx_slice,
                                rx_len: 0,
                                mode,
                                lsb_first: lsb,
                            });
                        } else {
                            return idx - 3; // Replay header when full payload arrives
                        }
                    } else {
                        return idx - 1;
                    }
                }

                // Data Shifting: Byte In (Read only)
                CMD_DATA_IN_BYTES_POS
                | CMD_DATA_IN_BYTES_NEG
                | CMD_DATA_IN_BYTES_LSB_POS
                | CMD_DATA_IN_BYTES_LSB_NEG => {
                    if idx + 2 <= len {
                        let count = ((input[idx + 1] as usize) << 8) | (input[idx] as usize);
                        let xfer_len = count + 1;
                        idx += 2;
                        let lsb = (cmd == CMD_DATA_IN_BYTES_LSB_POS) || (cmd == CMD_DATA_IN_BYTES_LSB_NEG);
                        let mode = if (cmd & 4) != 0 { MpsseSpiMode::Mode1 } else { MpsseSpiMode::Mode0 };
                        on_action(MpsseAction::SpiTransfer {
                            tx_data: &[],
                            rx_len: xfer_len,
                            mode,
                            lsb_first: lsb,
                        });
                    } else {
                        return idx - 1;
                    }
                }

                // Data Shifting: Full Duplex (Write Out & Read In)
                CMD_DATA_IN_OUT_BYTES_POS_NEG
                | CMD_DATA_IN_OUT_BYTES_NEG_POS
                | CMD_DATA_IN_OUT_BYTES_POS_POS
                | CMD_DATA_IN_OUT_BYTES_NEG_NEG
                | CMD_DATA_IN_OUT_BYTES_LSB_POS_NEG
                | CMD_DATA_IN_OUT_BYTES_LSB_NEG_POS => {
                    if idx + 2 <= len {
                        let count = ((input[idx + 1] as usize) << 8) | (input[idx] as usize);
                        let xfer_len = count + 1;
                        idx += 2;
                        if idx + xfer_len <= len {
                            let tx_slice = &input[idx..idx + xfer_len];
                            idx += xfer_len;
                            let lsb = (cmd == CMD_DATA_IN_OUT_BYTES_LSB_POS_NEG) || (cmd == CMD_DATA_IN_OUT_BYTES_LSB_NEG_POS);
                            let mode = match cmd {
                                CMD_DATA_IN_OUT_BYTES_NEG_POS => MpsseSpiMode::Mode0,
                                CMD_DATA_IN_OUT_BYTES_POS_NEG => MpsseSpiMode::Mode1,
                                _ => self.default_mode,
                            };
                            on_action(MpsseAction::SpiTransfer {
                                tx_data: tx_slice,
                                rx_len: xfer_len,
                                mode,
                                lsb_first: lsb,
                            });
                        } else {
                            return idx - 3;
                        }
                    } else {
                        return idx - 1;
                    }
                }

                // Data Shifting: Bit mode (Single byte of bits)
                CMD_DATA_OUT_BITS_NEG
                | CMD_DATA_OUT_BITS_POS
                | CMD_DATA_OUT_BITS_LSB_NEG
                | CMD_DATA_OUT_BITS_LSB_POS => {
                    if idx + 2 <= len {
                        // byte 0: bit count - 1 (0..7), byte 1: data byte
                        let _bit_count = (input[idx] & 7) + 1;
                        let tx_slice = &input[idx + 1..idx + 2];
                        idx += 2;
                        let lsb = (cmd == CMD_DATA_OUT_BITS_LSB_NEG) || (cmd == CMD_DATA_OUT_BITS_LSB_POS);
                        on_action(MpsseAction::SpiTransfer {
                            tx_data: tx_slice,
                            rx_len: 0,
                            mode: self.default_mode,
                            lsb_first: lsb,
                        });
                    } else {
                        return idx - 1;
                    }
                }

                CMD_DATA_IN_BITS_POS
                | CMD_DATA_IN_BITS_NEG
                | CMD_DATA_IN_BITS_LSB_POS
                | CMD_DATA_IN_BITS_LSB_NEG => {
                    if idx < len {
                        let _bit_count = (input[idx] & 7) + 1;
                        idx += 1;
                        let lsb = (cmd == CMD_DATA_IN_BITS_LSB_POS) || (cmd == CMD_DATA_IN_BITS_LSB_NEG);
                        on_action(MpsseAction::SpiTransfer {
                            tx_data: &[],
                            rx_len: 1,
                            mode: self.default_mode,
                            lsb_first: lsb,
                        });
                    } else {
                        return idx - 1;
                    }
                }

                // Bad command fallback
                bad => {
                    on_action(MpsseAction::BadCommand(bad));
                }
            }
        }

        idx
    }
}
