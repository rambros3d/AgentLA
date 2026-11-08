//! Zero-heap decoded event representations and timestamped transition records.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum ProtocolKind {
    Uart = 0,
    I2c = 1,
    Spi = 2,
    Can = 3,
}

impl ProtocolKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Uart => "uart",
            Self::I2c => "i2c",
            Self::Spi => "spi",
            Self::Can => "can",
        }
    }
}

/// A raw digital signal transition from the DMA / PIO sampling engine.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Transition {
    pub timestamp_ticks: u32,
    pub pin_states: u32,
}

/// Zero-heap decoded protocol event emitted by on-chip streaming decoders.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DecodedEvent {
    UartByte {
        timestamp_us: u32,
        byte: u8,
        parity_err: bool,
        framing_err: bool,
    },
    I2cTransaction {
        timestamp_us: u32,
        addr: u16,
        is_10bit: bool,
        is_read: bool,
        ack: bool,
        data_len: u8,
        data: [u8; 8],
    },
    SpiTransfer {
        timestamp_us: u32,
        mosi: [u8; 8],
        miso: [u8; 8],
        len: u8,
    },
    CanFrame {
        timestamp_us: u32,
        id: u32,
        is_extended: bool,
        rtr: bool,
        dlc: u8,
        data: [u8; 8],
        crc_ok: bool,
    },
    Error {
        timestamp_us: u32,
        kind: ProtocolKind,
        code: u8,
    },
}

impl DecodedEvent {
    pub fn timestamp_us(&self) -> u32 {
        match *self {
            Self::UartByte { timestamp_us, .. } => timestamp_us,
            Self::I2cTransaction { timestamp_us, .. } => timestamp_us,
            Self::SpiTransfer { timestamp_us, .. } => timestamp_us,
            Self::CanFrame { timestamp_us, .. } => timestamp_us,
            Self::Error { timestamp_us, .. } => timestamp_us,
        }
    }

    pub fn kind(&self) -> ProtocolKind {
        match *self {
            Self::UartByte { .. } => ProtocolKind::Uart,
            Self::I2cTransaction { .. } => ProtocolKind::I2c,
            Self::SpiTransfer { .. } => ProtocolKind::Spi,
            Self::CanFrame { .. } => ProtocolKind::Can,
            Self::Error { kind, .. } => kind,
        }
    }

    /// Serializes the event as a compact JSON string into the provided byte buffer without heap allocation.
    /// Returns the number of bytes written.
    pub fn format_json(&self, buf: &mut [u8]) -> usize {
        let mut cursor = 0usize;

        macro_rules! write_str {
            ($s:expr) => {{
                let bytes = $s.as_bytes();
                let len = bytes.len();
                if cursor + len <= buf.len() {
                    buf[cursor..cursor + len].copy_from_slice(bytes);
                    cursor += len;
                }
            }};
        }

        macro_rules! write_u32 {
            ($v:expr) => {{
                let mut temp = [0u8; 10];
                let mut val = $v;
                let mut i = 0;
                if val == 0 {
                    write_str!("0");
                } else {
                    while val > 0 {
                        temp[i] = b'0' + (val % 10) as u8;
                        val /= 10;
                        i += 1;
                    }
                    while i > 0 {
                        i -= 1;
                        if cursor < buf.len() {
                            buf[cursor] = temp[i];
                            cursor += 1;
                        }
                    }
                }
            }};
        }

        macro_rules! write_hex_byte {
            ($b:expr) => {{
                const HEX: &[u8; 16] = b"0123456789ABCDEF";
                if cursor + 4 <= buf.len() {
                    buf[cursor] = b'"';
                    buf[cursor + 1] = HEX[(($b >> 4) & 0x0F) as usize];
                    buf[cursor + 2] = HEX[($b & 0x0F) as usize];
                    buf[cursor + 3] = b'"';
                    cursor += 4;
                }
            }};
        }

        match *self {
            Self::UartByte { timestamp_us, byte, parity_err, framing_err } => {
                write_str!(r#"{"bus":"uart","t":"#);
                write_u32!(timestamp_us);
                write_str!(r#","byte":"#);
                write_hex_byte!(byte);
                write_str!(r#","perr":"#);
                write_str!(if parity_err { "true" } else { "false" });
                write_str!(r#","ferr":"#);
                write_str!(if framing_err { "true" } else { "false" });
                write_str!("}");
            }
            Self::I2cTransaction { timestamp_us, addr, is_10bit, is_read, ack, data_len, data } => {
                write_str!(r#"{"bus":"i2c","t":"#);
                write_u32!(timestamp_us);
                write_str!(r#","addr":"#);
                write_u32!(addr as u32);
                write_str!(r#","10bit":"#);
                write_str!(if is_10bit { "true" } else { "false" });
                write_str!(r#","rw":"#);
                write_str!(if is_read { r#""R""# } else { r#""W""# });
                write_str!(r#","ack":"#);
                write_str!(if ack { "true" } else { "false" });
                write_str!(r#","data":["#);
                let count = (data_len as usize).min(data.len());
                for (idx, &b) in data[..count].iter().enumerate() {
                    if idx > 0 {
                        write_str!(",");
                    }
                    write_hex_byte!(b);
                }
                write_str!("]}");
            }
            Self::SpiTransfer { timestamp_us, mosi, miso, len } => {
                write_str!(r#"{"bus":"spi","t":"#);
                write_u32!(timestamp_us);
                write_str!(r#","len":"#);
                write_u32!(len as u32);
                write_str!(r#","mosi":["#);
                let count = (len as usize).min(mosi.len());
                for (idx, &b) in mosi[..count].iter().enumerate() {
                    if idx > 0 { write_str!(","); }
                    write_hex_byte!(b);
                }
                write_str!(r#"],"miso":["#);
                for (idx, &b) in miso[..count].iter().enumerate() {
                    if idx > 0 { write_str!(","); }
                    write_hex_byte!(b);
                }
                write_str!("]}");
            }
            Self::CanFrame { timestamp_us, id, is_extended, rtr, dlc, data, crc_ok } => {
                write_str!(r#"{"bus":"can","t":"#);
                write_u32!(timestamp_us);
                write_str!(r#","id":"#);
                write_u32!(id);
                write_str!(r#","ext":"#);
                write_str!(if is_extended { "true" } else { "false" });
                write_str!(r#","rtr":"#);
                write_str!(if rtr { "true" } else { "false" });
                write_str!(r#","dlc":"#);
                write_u32!(dlc as u32);
                write_str!(r#","crc_ok":"#);
                write_str!(if crc_ok { "true" } else { "false" });
                write_str!(r#","data":["#);
                let count = (dlc as usize).min(data.len());
                for (idx, &b) in data[..count].iter().enumerate() {
                    if idx > 0 { write_str!(","); }
                    write_hex_byte!(b);
                }
                write_str!("]}");
            }
            Self::Error { timestamp_us, kind, code } => {
                write_str!(r#"{"bus":""#);
                write_str!(kind.as_str());
                write_str!(r#"","t":"#);
                write_u32!(timestamp_us);
                write_str!(r#","error":"#);
                write_u32!(code as u32);
                write_str!("}");
            }
        }

        cursor
    }
}
