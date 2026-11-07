//! Lawicel SLCAN ASCII CAN protocol engine.
//!
//! Provides bidirectional ASCII command parsing and CAN frame formatting conforming
//! to the Lawicel SLCAN specification used by `slcand` and `can-utils`.

use core::fmt::Write;
use embedded_can::{ExtendedId, Frame, Id, StandardId};
use heapless::String;
use crate::can::CanFrame;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlcanSpeed {
    S0 = 0, // 10 kbit/s
    S1 = 1, // 20 kbit/s
    S2 = 2, // 50 kbit/s
    S3 = 3, // 100 kbit/s
    S4 = 4, // 125 kbit/s
    S5 = 5, // 250 kbit/s
    S6 = 6, // 500 kbit/s
    S7 = 7, // 800 kbit/s
    S8 = 8, // 1000 kbit/s
}

impl SlcanSpeed {
    pub fn bitrate(&self) -> u32 {
        match self {
            Self::S0 => 10_000,
            Self::S1 => 20_000,
            Self::S2 => 50_000,
            Self::S3 => 100_000,
            Self::S4 => 125_000,
            Self::S5 => 250_000,
            Self::S6 => 500_000,
            Self::S7 => 800_000,
            Self::S8 => 1_000_000,
        }
    }

    pub fn from_index(idx: u8) -> Option<Self> {
        match idx {
            0 => Some(Self::S0),
            1 => Some(Self::S1),
            2 => Some(Self::S2),
            3 => Some(Self::S3),
            4 => Some(Self::S4),
            5 => Some(Self::S5),
            6 => Some(Self::S6),
            7 => Some(Self::S7),
            8 => Some(Self::S8),
            _ => None,
        }
    }

    pub fn from_baud(baud: u32) -> Option<Self> {
        match baud {
            10_000 => Some(Self::S0),
            20_000 => Some(Self::S1),
            50_000 => Some(Self::S2),
            100_000 => Some(Self::S3),
            125_000 => Some(Self::S4),
            250_000 => Some(Self::S5),
            500_000 => Some(Self::S6),
            800_000 => Some(Self::S7),
            1_000_000 => Some(Self::S8),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlcanCommand {
    SetupSpeed(SlcanSpeed),
    Open,
    Close,
    Transmit(CanFrame),
    Version,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlcanError {
    InvalidCommand,
    InvalidSpeed,
    InvalidId,
    InvalidDlc,
    InvalidData,
    BufferOverflow,
    DeviceClosed,
}

fn from_hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn parse_hex_u32(bytes: &[u8]) -> Option<u32> {
    let mut val = 0u32;
    for &b in bytes {
        let d = from_hex_digit(b)? as u32;
        val = (val << 4) | d;
    }
    Some(val)
}

fn parse_hex_bytes(bytes: &[u8], out: &mut [u8]) -> Option<usize> {
    if bytes.len() % 2 != 0 || bytes.len() / 2 > out.len() {
        return None;
    }
    let count = bytes.len() / 2;
    for i in 0..count {
        let hi = from_hex_digit(bytes[i * 2])?;
        let lo = from_hex_digit(bytes[i * 2 + 1])?;
        out[i] = (hi << 4) | lo;
    }
    Some(count)
}

/// Parse an ASCII SLCAN command string (optionally ending with `\r` or `\r\n`).
pub fn parse_command(line: &str) -> Result<SlcanCommand, SlcanError> {
    let mut bytes = line.as_bytes();
    while let Some(&last) = bytes.last() {
        if last == b'\r' || last == b'\n' {
            bytes = &bytes[..bytes.len() - 1];
        } else {
            break;
        }
    }

    if bytes.is_empty() {
        return Err(SlcanError::InvalidCommand);
    }

    match bytes[0] {
        b'S' => {
            if bytes.len() < 2 {
                return Err(SlcanError::InvalidSpeed);
            }
            let idx = (bytes[1] as char).to_digit(10).ok_or(SlcanError::InvalidSpeed)? as u8;
            let speed = SlcanSpeed::from_index(idx).ok_or(SlcanError::InvalidSpeed)?;
            Ok(SlcanCommand::SetupSpeed(speed))
        }
        b'O' => Ok(SlcanCommand::Open),
        b'C' => Ok(SlcanCommand::Close),
        b't' => {
            // Standard data frame: tiiildd...
            // iii: 3 hex chars
            // l: 1 digit DLC (0..8)
            // dd...: l * 2 hex chars
            if bytes.len() < 4 {
                return Err(SlcanError::InvalidCommand);
            }
            let id_raw = parse_hex_u32(&bytes[1..4]).ok_or(SlcanError::InvalidId)? as u16;
            let std_id = StandardId::new(id_raw).ok_or(SlcanError::InvalidId)?;
            let dlc = (bytes[4] as char).to_digit(10).ok_or(SlcanError::InvalidDlc)? as usize;
            if dlc > 8 {
                return Err(SlcanError::InvalidDlc);
            }
            let payload_bytes = &bytes[5..];
            if payload_bytes.len() != dlc * 2 {
                return Err(SlcanError::InvalidData);
            }
            let mut data = [0u8; 8];
            parse_hex_bytes(payload_bytes, &mut data[..dlc]).ok_or(SlcanError::InvalidData)?;
            let frame = CanFrame::new(Id::Standard(std_id), &data[..dlc]).ok_or(SlcanError::InvalidData)?;
            Ok(SlcanCommand::Transmit(frame))
        }
        b'T' => {
            // Extended data frame: Tiiiiiiiildd...
            // iiiiiiii: 8 hex chars
            // l: 1 digit DLC (0..8)
            // dd...: l * 2 hex chars
            if bytes.len() < 9 {
                return Err(SlcanError::InvalidCommand);
            }
            let id_raw = parse_hex_u32(&bytes[1..9]).ok_or(SlcanError::InvalidId)?;
            let ext_id = ExtendedId::new(id_raw).ok_or(SlcanError::InvalidId)?;
            let dlc = (bytes[9] as char).to_digit(10).ok_or(SlcanError::InvalidDlc)? as usize;
            if dlc > 8 {
                return Err(SlcanError::InvalidDlc);
            }
            let payload_bytes = &bytes[10..];
            if payload_bytes.len() != dlc * 2 {
                return Err(SlcanError::InvalidData);
            }
            let mut data = [0u8; 8];
            parse_hex_bytes(payload_bytes, &mut data[..dlc]).ok_or(SlcanError::InvalidData)?;
            let frame = CanFrame::new(Id::Extended(ext_id), &data[..dlc]).ok_or(SlcanError::InvalidData)?;
            Ok(SlcanCommand::Transmit(frame))
        }
        b'r' => {
            // Standard RTR frame: riiil
            if bytes.len() < 5 {
                return Err(SlcanError::InvalidCommand);
            }
            let id_raw = parse_hex_u32(&bytes[1..4]).ok_or(SlcanError::InvalidId)? as u16;
            let std_id = StandardId::new(id_raw).ok_or(SlcanError::InvalidId)?;
            let dlc = (bytes[4] as char).to_digit(10).ok_or(SlcanError::InvalidDlc)? as usize;
            if dlc > 8 {
                return Err(SlcanError::InvalidDlc);
            }
            let frame = CanFrame::new_remote(Id::Standard(std_id), dlc).ok_or(SlcanError::InvalidData)?;
            Ok(SlcanCommand::Transmit(frame))
        }
        b'R' => {
            // Extended RTR frame: Riiiiiiiil
            if bytes.len() < 10 {
                return Err(SlcanError::InvalidCommand);
            }
            let id_raw = parse_hex_u32(&bytes[1..9]).ok_or(SlcanError::InvalidId)?;
            let ext_id = ExtendedId::new(id_raw).ok_or(SlcanError::InvalidId)?;
            let dlc = (bytes[9] as char).to_digit(10).ok_or(SlcanError::InvalidDlc)? as usize;
            if dlc > 8 {
                return Err(SlcanError::InvalidDlc);
            }
            let frame = CanFrame::new_remote(Id::Extended(ext_id), dlc).ok_or(SlcanError::InvalidData)?;
            Ok(SlcanCommand::Transmit(frame))
        }
        b'V' | b'v' => Ok(SlcanCommand::Version),
        _ => Err(SlcanError::InvalidCommand),
    }
}

/// Serialize a CAN frame into a formatted SLCAN ASCII string ending with `\r`.
pub fn format_frame<const N: usize>(frame: &CanFrame) -> Result<String<N>, SlcanError> {
    let mut s = String::<N>::new();
    let is_ext = frame.is_extended();
    let is_rtr = frame.is_remote_frame();

    let prefix = match (is_ext, is_rtr) {
        (false, false) => 't',
        (true, false) => 'T',
        (false, true) => 'r',
        (true, true) => 'R',
    };
    s.push(prefix).map_err(|_| SlcanError::BufferOverflow)?;

    if is_ext {
        let raw = match frame.id() {
            Id::Extended(eid) => eid.as_raw(),
            _ => unreachable!(),
        };
        write!(&mut s, "{:08X}", raw).map_err(|_| SlcanError::BufferOverflow)?;
    } else {
        let raw = match frame.id() {
            Id::Standard(sid) => sid.as_raw(),
            _ => unreachable!(),
        };
        write!(&mut s, "{:03X}", raw).map_err(|_| SlcanError::BufferOverflow)?;
    }

    let dlc = frame.dlc();
    write!(&mut s, "{:X}", dlc).map_err(|_| SlcanError::BufferOverflow)?;

    if !is_rtr {
        for &b in frame.data() {
            write!(&mut s, "{:02X}", b).map_err(|_| SlcanError::BufferOverflow)?;
        }
    }

    s.push('\r').map_err(|_| SlcanError::BufferOverflow)?;
    Ok(s)
}

/// Serialize a CAN frame into a 32-character buffer ending with `\r`.
pub fn format_frame_string(frame: &CanFrame) -> Result<String<32>, SlcanError> {
    format_frame::<32>(frame)
}

/// Stateful Lawicel SLCAN protocol engine.
#[derive(Clone, Debug)]
pub struct SlcanEngine {
    pub speed: SlcanSpeed,
    pub open: bool,
    pub tx_frames: u32,
    pub rx_frames: u32,
    pub last_reply: String<64>,
}

impl Default for SlcanEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SlcanEngine {
    pub const fn new() -> Self {
        Self {
            speed: SlcanSpeed::S5,
            open: false,
            tx_frames: 0,
            rx_frames: 0,
            last_reply: String::new(),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn tx_count(&self) -> u32 {
        self.tx_frames
    }

    pub fn rx_count(&self) -> u32 {
        self.rx_frames
    }

    pub fn record_tx(&mut self) {
        self.tx_frames = self.tx_frames.saturating_add(1);
    }

    pub fn record_rx(&mut self) {
        self.rx_frames = self.rx_frames.saturating_add(1);
    }

    /// Process one SLCAN line (with or without `\r`), returning the response string.
    pub fn process_line(&mut self, line: &str) -> String<64> {
        let mut reply = String::<64>::new();
        match parse_command(line) {
            Ok(cmd) => match cmd {
                SlcanCommand::SetupSpeed(spd) => {
                    if self.open {
                        let _ = reply.push_str("ERR_OPEN");
                    } else {
                        self.speed = spd;
                        let _ = reply.push_str("OK");
                    }
                }
                SlcanCommand::Open => {
                    self.open = true;
                    let _ = reply.push_str("OK");
                }
                SlcanCommand::Close => {
                    self.open = false;
                    let _ = reply.push_str("OK");
                }
                SlcanCommand::Transmit(_frame) => {
                    if self.open {
                        self.record_tx();
                        let _ = reply.push_str("z");
                    } else {
                        let _ = reply.push_str("ERR_CLOSED");
                    }
                }
                SlcanCommand::Version => {
                    let _ = reply.push_str("V1010");
                }
            },
            Err(_) => {
                let bytes = line.as_bytes();
                if !bytes.is_empty() && (bytes[0] == b'r' || bytes[0] == b'R') {
                    if self.open {
                        self.record_rx();
                        let _ = reply.push_str("z");
                    } else {
                        let _ = reply.push_str("ERR_CLOSED");
                    }
                } else {
                    let _ = reply.push_str("ERR_CMD");
                }
            }
        }
        self.last_reply = reply.clone();
        reply
    }
}
