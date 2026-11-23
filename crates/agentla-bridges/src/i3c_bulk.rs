//! AgentLA Binary I3C Bulk Protocol engine.
//!
//! Provides `#![no_std]` binary framing and command dispatching over USB Bulk
//! endpoints for high-speed MIPI I3C SDR operations. Bypasses REST latency to
//! allow full 12.5 MHz bus transfers and high-speed BareBoot/Katapult flashing.

#![allow(dead_code)]

pub const I3C_BULK_MAGIC: u8 = 0x3C;
pub const I3C_HEADER_SIZE: usize = 8;

// Command Opcodes (Request)
pub const I3C_OP_SET_BUS_SPEED: u8 = 0x01; // Payload: 4-byte u32 LE (frequency in kHz)
pub const I3C_OP_TARGET_RESET: u8  = 0x02; // Emit target reset pattern on SCL/SDA
pub const I3C_OP_RSTDAA: u8        = 0x03; // Broadcast CCC 0x06 (Reset DAA)
pub const I3C_OP_ENTDAA: u8        = 0x04; // Broadcast CCC 0x07 (Enter DAA), payload: target assigned addr
pub const I3C_OP_CCC_BROADCAST: u8 = 0x05; // Broadcast CCC; addr = CCC code, payload = data
pub const I3C_OP_CCC_DIRECT: u8    = 0x06; // Direct CCC; addr = target dynamic addr, payload = [CCC, data...]
pub const I3C_OP_PRIVATE_WRITE: u8 = 0x07; // Push-pull write to target addr
pub const I3C_OP_PRIVATE_READ: u8  = 0x08; // Push-pull read from target addr (payload_len = bytes to read)
pub const I3C_OP_PRIVATE_WRITE_READ: u8 = 0x09; // Write payload, repeated start, read rx_len bytes
pub const I3C_OP_IBI_CONFIG: u8    = 0x10; // Configure IBI ACK behavior

// Event Opcodes (Asynchronous from device to host)
pub const I3C_EVT_IBI_NOTIFICATION: u8 = 0x80;

// Status Codes (Response)
pub const I3C_STATUS_OK: u8          = 0x00;
pub const I3C_STATUS_IBI: u8         = 0x01;
pub const I3C_STATUS_NAK_ARBHDR: u8  = 0x02; // Arbitration header 0x7E was NAKed
pub const I3C_STATUS_NAK_ADDR: u8    = 0x03; // Target dynamic address was NAKed
pub const I3C_STATUS_PARITY_ERR: u8  = 0x04; // T-bit parity mismatch during read
pub const I3C_STATUS_TIMEOUT: u8     = 0x05;
pub const I3C_STATUS_INVALID_PARAM: u8 = 0x06;

// Flag bits
pub const I3C_FLAG_REPEATED_START: u8 = 1 << 0;
pub const I3C_FLAG_EMIT_STOP: u8      = 1 << 1;
pub const I3C_FLAG_OPEN_DRAIN: u8     = 1 << 2;
pub const I3C_FLAG_PUSH_PULL: u8      = 1 << 3;

/// Header of an AgentLA I3C Bulk Frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I3cBulkHeader {
    pub magic: u8,
    pub opcode: u8,
    pub flags: u8,
    pub seq: u8,
    pub payload_len: u16,
    pub addr: u8,
    pub status: u8,
}

impl Default for I3cBulkHeader {
    fn default() -> Self {
        Self {
            magic: I3C_BULK_MAGIC,
            opcode: 0,
            flags: I3C_FLAG_EMIT_STOP | I3C_FLAG_PUSH_PULL,
            seq: 0,
            payload_len: 0,
            addr: 0,
            status: I3C_STATUS_OK,
        }
    }
}

impl I3cBulkHeader {
    pub const fn new(opcode: u8, seq: u8, addr: u8, payload_len: u16) -> Self {
        Self {
            magic: I3C_BULK_MAGIC,
            opcode,
            flags: I3C_FLAG_EMIT_STOP | I3C_FLAG_PUSH_PULL,
            seq,
            payload_len,
            addr,
            status: I3C_STATUS_OK,
        }
    }

    /// Parse an 8-byte header from `bytes`.
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < I3C_HEADER_SIZE {
            return None;
        }
        if bytes[0] != I3C_BULK_MAGIC {
            return None;
        }

        let magic = bytes[0];
        let opcode = bytes[1];
        let flags = bytes[2];
        let seq = bytes[3];
        let payload_len = u16::from_le_bytes([bytes[4], bytes[5]]);
        let addr = bytes[6];
        let status = bytes[7];

        Some(Self {
            magic,
            opcode,
            flags,
            seq,
            payload_len,
            addr,
            status,
        })
    }

    /// Serialize header into an 8-byte destination slice.
    pub fn serialize(&self, dest: &mut [u8]) -> bool {
        if dest.len() < I3C_HEADER_SIZE {
            return false;
        }
        dest[0] = self.magic;
        dest[1] = self.opcode;
        dest[2] = self.flags;
        dest[3] = self.seq;
        let len_bytes = self.payload_len.to_le_bytes();
        dest[4] = len_bytes[0];
        dest[5] = len_bytes[1];
        dest[6] = self.addr;
        dest[7] = self.status;
        true
    }
}

/// Parsed or staged I3C Bulk Frame referencing data payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct I3cBulkFrame<'a> {
    pub header: I3cBulkHeader,
    pub payload: &'a [u8],
}

impl<'a> I3cBulkFrame<'a> {
    pub fn new(header: I3cBulkHeader, payload: &'a [u8]) -> Self {
        Self { header, payload }
    }

    /// Parse complete frame from `bytes`.
    pub fn parse(bytes: &'a [u8]) -> Option<Self> {
        let header = I3cBulkHeader::parse(bytes)?;
        let total_len = I3C_HEADER_SIZE + (header.payload_len as usize);
        if bytes.len() < total_len {
            return None;
        }
        let payload = &bytes[I3C_HEADER_SIZE..total_len];
        Some(Self { header, payload })
    }

    /// Serialize complete frame into `dest`. Returns bytes written.
    pub fn serialize(&self, dest: &mut [u8]) -> Option<usize> {
        let total_len = I3C_HEADER_SIZE + self.payload.len();
        if dest.len() < total_len {
            return None;
        }
        self.header.serialize(&mut dest[..I3C_HEADER_SIZE]);
        dest[I3C_HEADER_SIZE..total_len].copy_from_slice(self.payload);
        Some(total_len)
    }

    /// Build a successful response frame matching this request's sequence ID.
    pub fn make_response(
        seq: u8,
        opcode: u8,
        status: u8,
        addr: u8,
        payload: &'a [u8],
        dest: &mut [u8],
    ) -> Option<usize> {
        let total_len = I3C_HEADER_SIZE + payload.len();
        if dest.len() < total_len {
            return None;
        }
        let header = I3cBulkHeader {
            magic: I3C_BULK_MAGIC,
            opcode,
            flags: 0,
            seq,
            payload_len: payload.len() as u16,
            addr,
            status,
        };
        header.serialize(&mut dest[..I3C_HEADER_SIZE]);
        dest[I3C_HEADER_SIZE..total_len].copy_from_slice(payload);
        Some(total_len)
    }
}
