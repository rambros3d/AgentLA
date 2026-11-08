//! OpenBench Logic Sniffer (OLS / Sump) protocol engine.
//! Supports Sigrok/Pulseview desktop logic analyzer client over TCP (:5555).

pub const CMD_RESET: u8 = 0x00;
pub const CMD_RUN: u8 = 0x01;
pub const CMD_ID: u8 = 0x02;
pub const CMD_METADATA: u8 = 0x04;

pub const CMD_SET_DIVIDER: u8 = 0x80;
pub const CMD_SET_READ_COUNT: u8 = 0x81;
pub const CMD_SET_FLAGS: u8 = 0x82;

pub const CMD_SET_TRIG_MASK_0: u8 = 0xC0;
pub const CMD_SET_TRIG_MASK_1: u8 = 0xC1;
pub const CMD_SET_TRIG_MASK_2: u8 = 0xC2;
pub const CMD_SET_TRIG_MASK_3: u8 = 0xC3;

pub const CMD_SET_TRIG_VAL_0: u8 = 0xC4;
pub const CMD_SET_TRIG_VAL_1: u8 = 0xC5;
pub const CMD_SET_TRIG_VAL_2: u8 = 0xC6;
pub const CMD_SET_TRIG_VAL_3: u8 = 0xC7;

pub const OLS_ID_RESPONSE: &[u8; 4] = b"1ALS";

/// OLS Metadata TLV keys
pub const TLV_END: u8 = 0x00;
pub const TLV_DEVICE_NAME: u8 = 0x01;
pub const TLV_FPGA_VERSION: u8 = 0x02;
pub const TLV_ANCILLARY_VERSION: u8 = 0x03;
pub const TLV_MAX_SAMPLE_RATE: u8 = 0x20;
pub const TLV_SAMPLE_MEMORY: u8 = 0x21;
pub const TLV_DYNAMIC_MEMORY: u8 = 0x22;
pub const TLV_MAX_EXT_SAMPLE_RATE: u8 = 0x23;
pub const TLV_NUM_PROBES: u8 = 0x24;
pub const TLV_PROTOCOL_FLAGS: u8 = 0x40;

/// Parsed OLS Command
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OlsCommand {
    Reset,
    Run,
    QueryId,
    QueryMetadata,
    SetDivider { divider: u32, sample_rate_hz: u32 },
    SetReadCount { read_count: u32, delay_count: u32 },
    SetFlags { flags: u32 },
    SetTriggerMask { stage: u8, mask: u32 },
    SetTriggerValue { stage: u8, value: u32 },
}

/// OLS state machine parser for 1-byte commands and 5-byte parameter packets
#[derive(Debug, Default)]
pub struct OlsParser {
    param_buf: [u8; 4],
    param_cmd: u8,
    param_idx: u8,
}

impl OlsParser {
    pub const fn new() -> Self {
        Self {
            param_buf: [0; 4],
            param_cmd: 0,
            param_idx: 0,
        }
    }

    pub fn reset(&mut self) {
        self.param_cmd = 0;
        self.param_idx = 0;
    }

    /// Process a single byte from the client stream.
    /// Returns `Some(OlsCommand)` when a complete command is decoded.
    pub fn process_byte(&mut self, b: u8) -> Option<OlsCommand> {
        if self.param_cmd != 0 {
            // We are gathering the 4 parameter bytes
            self.param_buf[self.param_idx as usize] = b;
            self.param_idx += 1;
            if self.param_idx == 4 {
                let cmd = self.param_cmd;
                let val = u32::from_le_bytes(self.param_buf);
                self.reset();
                return Some(Self::decode_param_cmd(cmd, val));
            }
            return None;
        }

        // 1-byte command dispatch
        match b {
            CMD_RESET => Some(OlsCommand::Reset),
            CMD_RUN => Some(OlsCommand::Run),
            CMD_ID => Some(OlsCommand::QueryId),
            CMD_METADATA => Some(OlsCommand::QueryMetadata),
            CMD_SET_DIVIDER | CMD_SET_READ_COUNT | CMD_SET_FLAGS
            | 0xC0..=0xC7 => {
                // Parameter command: expect 4 payload bytes (little-endian)
                self.param_cmd = b;
                self.param_idx = 0;
                None
            }
            _ => None, // Unknown command ignored
        }
    }

    fn decode_param_cmd(cmd: u8, val: u32) -> OlsCommand {
        match cmd {
            CMD_SET_DIVIDER => {
                let divider = val & 0x00FF_FFFF;
                let sample_rate_hz = if divider == 0 {
                    100_000_000
                } else {
                    100_000_000 / (divider + 1)
                };
                OlsCommand::SetDivider { divider, sample_rate_hz }
            }
            CMD_SET_READ_COUNT => {
                let read_count = (val & 0xFFFF) * 4;
                let delay_count = ((val >> 16) & 0xFFFF) * 4;
                OlsCommand::SetReadCount { read_count, delay_count }
            }
            CMD_SET_FLAGS => OlsCommand::SetFlags { flags: val },
            0xC0..=0xC3 => OlsCommand::SetTriggerMask { stage: cmd - 0xC0, mask: val },
            0xC4..=0xC7 => OlsCommand::SetTriggerValue { stage: cmd - 0xC4, value: val },
            _ => OlsCommand::Reset,
        }
    }
}

/// Generates standard OLS metadata TLV block.
/// Returns number of bytes written to `dst`.
pub fn generate_ols_metadata(dst: &mut [u8]) -> usize {
    let mut offset = 0;

    macro_rules! write_str {
        ($tag:expr, $str:expr) => {
            if offset + 2 + $str.len() <= dst.len() {
                dst[offset] = $tag;
                offset += 1;
                for b in $str.as_bytes() {
                    dst[offset] = *b;
                    offset += 1;
                }
                dst[offset] = 0; // null terminator
                offset += 1;
            }
        };
    }

    macro_rules! write_u32_be {
        ($tag:expr, $val:expr) => {
            if offset + 5 <= dst.len() {
                dst[offset] = $tag;
                offset += 1;
                let be = ($val as u32).to_be_bytes();
                dst[offset..offset + 4].copy_from_slice(&be);
                offset += 4;
            }
        };
    }

    macro_rules! write_u8 {
        ($tag:expr, $val:expr) => {
            if offset + 2 <= dst.len() {
                dst[offset] = $tag;
                offset += 1;
                dst[offset] = $val;
                offset += 1;
            }
        };
    }

    // 1. Device Name
    write_str!(TLV_DEVICE_NAME, "AgentLA RP2350 Logic Bench");
    // 2. Firmware Version
    write_str!(TLV_FPGA_VERSION, "v1.0.0-rust");
    // 3. Ancillary Version
    write_str!(TLV_ANCILLARY_VERSION, "OLS/Sump 1.0");
    // 4. Max Sample Rate (100 MHz = 0x05F5E100)
    write_u32_be!(TLV_MAX_SAMPLE_RATE, 100_000_000);
    // 5. Sample Memory Size (1,048,576 samples)
    write_u32_be!(TLV_SAMPLE_MEMORY, 1_048_576);
    // 6. Number of usable probes (16 channels)
    write_u8!(TLV_NUM_PROBES, 16);
    // 7. Protocol Flags (bit 2 = RLE supported)
    write_u8!(TLV_PROTOCOL_FLAGS, 0x04);
    // 8. End marker
    if offset < dst.len() {
        dst[offset] = TLV_END;
        offset += 1;
    }

    offset
}

/// Encode 8-bit samples into standard SUMP RLE wire format.
/// In SUMP RLE:
/// - Sample value byte has bit 7 = 0 (`0xxxxxxx`).
/// - Run count byte has bit 7 = 1 (`1xxxxxxx`, count = value & 0x7F).
///
/// Returns total bytes written to `dst`.
pub fn sump_rle_encode_8bit(src: &[u8], dst: &mut [u8]) -> usize {
    let mut in_idx = 0;
    let mut out_idx = 0;

    while in_idx < src.len() {
        let val = src[in_idx] & 0x7F; // 7-bit sample value
        let mut count: usize = 0;

        while (in_idx + 1 + count) < src.len()
            && (src[in_idx + 1 + count] & 0x7F) == val
            && count < 0x7F
        {
            count += 1;
        }

        if out_idx >= dst.len() {
            break;
        }
        dst[out_idx] = val; // bit 7 is 0
        out_idx += 1;

        if count > 0 {
            if out_idx >= dst.len() {
                break;
            }
            dst[out_idx] = 0x80 | (count as u8); // bit 7 is 1
            out_idx += 1;
        }

        in_idx += 1 + count;
    }

    out_idx
}
