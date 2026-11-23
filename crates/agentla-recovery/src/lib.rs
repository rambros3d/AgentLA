//! Hardware-independent helpers for the AgentLA recovery bootloader.
//!
//! Kept free of any ARM / Embassy dependencies so the pure logic (CRC32,
//! firmware-header validation, vector validation) can be unit-tested on the
//! host with `cargo test -p agentla-recovery --lib --target <host-triple>`.

#![no_std]

use agentla_boot_proto::FirmwareHeader;

/// XIP base of this recovery bootloader (flash 0x00000000).
pub const RECOVERY_BASE_XIP: u32 = 0x1000_0000;
/// Size of the recovery bootloader slot: 288 KB (flash 0x00000000..0x00048000).
pub const RECOVERY_SLOT_SIZE: u32 = 0x0004_8000;
/// XIP base of the single main application slot (flash 0x00048000).
pub const APP_BASE_XIP: u32 = 0x1004_8000;
/// Flash offset of the main application slot.
pub const APP_BASE_FLASH: u32 = 0x0004_8000;
/// Size of the main application slot: 1 MB, ending at flash 0x00148000.
pub const APP_SLOT_SIZE: u32 = 0x0010_0000;
/// End of the main application slot in flash coordinates (never write at or beyond this).
pub const APP_SLOT_END_FLASH: u32 = APP_BASE_FLASH + APP_SLOT_SIZE;
/// USER_FS flash offset. The only filesystem this crate mounts.
pub const USER_FS_OFFSET: u32 = 0x0024_8000;
/// USER_FS blocks: 1760 KB / 4 KB = 440.
pub const USER_BLOCK_COUNT: usize = 440;
/// RP2350 SRAM range used for the SP sanity check.
pub const RAM_BASE: u32 = 0x2000_0000;
pub const RAM_END: u32 = 0x2008_2000;

/// Size of the `AGLA_FW\0` firmware container header.
pub const HEADER_SIZE: usize = 64;
/// Candidate firmware file inside USER_FS.
pub const UPDATE_PATH: &str = "/ota/update.bin";
/// ASCII result file written by the recovery app.
pub const STATUS_PATH: &str = "/ota/status.txt";

/// Reflected IEEE 802.3 CRC-32 (polynomial 0xEDB88320), streaming variant.
pub struct Crc32(u32);

impl Crc32 {
    pub const fn new() -> Self {
        Self(0xFFFF_FFFF)
    }

    pub fn update(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.push(b);
        }
    }

    pub fn push(&mut self, b: u8) {
        self.0 ^= b as u32;
        for _ in 0..8 {
            self.0 = if self.0 & 1 != 0 {
                (self.0 >> 1) ^ 0xEDB8_8320
            } else {
                self.0 >> 1
            };
        }
    }

    pub fn finish(self) -> u32 {
        !self.0
    }
}

/// Default Crc32 (empty input), provided for convenience.
impl Default for Crc32 {
    fn default() -> Self {
        Self::new()
    }
}

/// Standalone CRC-32 over a byte slice, matching `agentla_boot_proto::crc32`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = Crc32::new();
    crc.update(bytes);
    crc.finish()
}

/// Validate the initial stack pointer and reset vector of an application image.
///
/// * `sp` must point inside RP2350 SRAM.
/// * `pc` must point inside the main app slot and carry the Thumb bit (bit 0).
pub fn valid_vector(sp: u32, pc: u32) -> bool {
    sp >= RAM_BASE
        && sp <= RAM_END
        && pc >= APP_BASE_XIP
        && pc < APP_BASE_XIP + APP_SLOT_SIZE
        && (pc & 1) == 1
}

/// Parse a 64-byte `AGLA_FW\0` header (magic + header CRC are checked).
pub fn parse_header(bytes: &[u8]) -> Option<FirmwareHeader> {
    FirmwareHeader::from_bytes(bytes)
}

/// Full policy check: valid header, supported board, sane payload size.
pub fn header_valid(hdr: &FirmwareHeader) -> bool {
    hdr.is_valid()
        && (hdr.matches_board(b"PICO2") || hdr.matches_board(b"BAKUPROB"))
        && hdr.payload_size > HEADER_SIZE as u32
        && hdr.payload_size <= APP_SLOT_SIZE
}

/// Round a byte count up to the next 4096-byte sector boundary.
pub const fn align_up_4096(n: usize) -> usize {
    (n + 4095) & !4095
}

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_known_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        let mut stream = Crc32::new();
        stream.update(b"1234");
        stream.update(b"56789");
        assert_eq!(stream.finish(), 0xCBF4_3926);
    }

    #[test]
    fn header_parse_roundtrip_and_crc() {
        let hdr = FirmwareHeader::new(b"PICO2", b"1.0.0", 0, 4096, 0xDEAD_BEEF);
        let bytes = hdr.as_bytes();
        assert_eq!(bytes.len(), HEADER_SIZE);
        let parsed = parse_header(bytes).expect("valid header must parse");
        assert!(header_valid(&parsed));
        assert_eq!(parsed.payload_size, 4096);
        assert_eq!(parsed.payload_crc, 0xDEAD_BEEF);

        // Corrupting any header byte must break the header CRC.
        for idx in [0usize, 8, 36, 40, 59] {
            let mut bad = [0u8; HEADER_SIZE];
            bad.copy_from_slice(bytes);
            bad[idx] ^= 1;
            assert!(parse_header(&bad).is_none(), "byte {} corruption undetected", idx);
        }
    }

    #[test]
    fn header_board_and_size_policy() {
        assert!(header_valid(&FirmwareHeader::new(
            b"PICO2", b"1.0", 0, 1000, 0
        )));
        assert!(header_valid(&FirmwareHeader::new(
            b"BAKUPROB", b"1.0", 0, 1000, 0
        )));
        assert!(!header_valid(&FirmwareHeader::new(
            b"OTHER123", b"1.0", 0, 1000, 0
        )));
        // payload_size must be > 64 and <= 1 MB.
        assert!(!header_valid(&FirmwareHeader::new(
            b"PICO2", b"1.0", 0, 64, 0
        )));
        assert!(!header_valid(&FirmwareHeader::new(
            b"PICO2", b"1.0", 0, APP_SLOT_SIZE + 1, 0
        )));
    }

    #[test]
    fn vector_validation_rules() {
        assert!(valid_vector(0x2000_8000, APP_BASE_XIP + 0x101));
        assert!(valid_vector(RAM_BASE, APP_BASE_XIP | 1));
        assert!(valid_vector(RAM_END, APP_BASE_XIP + APP_SLOT_SIZE - 1));
        // SP outside SRAM.
        assert!(!valid_vector(0x1FFF_FFFF, APP_BASE_XIP + 1));
        assert!(!valid_vector(0x2008_2001, APP_BASE_XIP + 1));
        // PC lacks Thumb bit / lies outside the app slot.
        assert!(!valid_vector(0x2000_8000, APP_BASE_XIP));
        assert!(!valid_vector(0x2000_8000, RECOVERY_BASE_XIP | 1));
        assert!(!valid_vector(
            0x2000_8000,
            APP_BASE_XIP + APP_SLOT_SIZE + 1
        ));
    }

    #[test]
    fn align_up_sector_boundary() {
        assert_eq!(align_up_4096(0), 0);
        assert_eq!(align_up_4096(1), 4096);
        assert_eq!(align_up_4096(4096), 4096);
        assert_eq!(align_up_4096(4097), 8192);
        assert_eq!(align_up_4096(APP_SLOT_SIZE as usize), APP_SLOT_SIZE as usize);
    }
}
