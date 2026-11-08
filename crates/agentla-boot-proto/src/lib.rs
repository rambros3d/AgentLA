#![no_std]

/// Standard IEEE 802.3 CRC-32 polynomial (0xEDB88320 in reversed representation).
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

pub const AGLA_BOOT_MAGIC: u32 = 0x4147_4C41; // "AGLA"
pub const AGLA_FW_MAGIC: [u8; 8] = *b"AGLA_FW\0";
pub const BOOT_TABLE_VERSION: u32 = 1;

pub const BOOT_SECTOR_0_OFFSET: u32 = 0x0000_2000;
pub const BOOT_SECTOR_1_OFFSET: u32 = 0x0000_3000;
pub const BOOT_SECTOR_SIZE: usize = 4096;

/// Execution slot identifier.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SlotId {
    SlotA = 0,
    SlotB = 1,
}

impl SlotId {
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::SlotA),
            1 => Some(Self::SlotB),
            _ => None,
        }
    }

    pub const fn other(self) -> Self {
        match self {
            Self::SlotA => Self::SlotB,
            Self::SlotB => Self::SlotA,
        }
    }

    /// Flash offset from start of RP2350 flash
    pub const fn flash_offset(self) -> u32 {
        match self {
            Self::SlotA => 0x0000_4000,
            Self::SlotB => 0x0010_4000,
        }
    }

    /// Memory-mapped XIP address (Flash base 0x10000000)
    pub const fn xip_address(self) -> u32 {
        0x1000_0000 + self.flash_offset()
    }
}

/// Boot state machine status.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BootState {
    Normal = 0,
    TestingCandidate = 1,
    RolledBack = 2,
}

impl BootState {
    pub const fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::TestingCandidate,
            2 => Self::RolledBack,
            _ => Self::Normal,
        }
    }
}

/// Boot metadata table (64 bytes aligned).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BootTable {
    pub magic: u32,
    pub version: u32,
    pub sequence: u32,
    pub active_slot: u8,
    pub candidate_slot: u8,
    pub boot_attempts: u8,
    pub state: u8,
    pub slot_a_crc: u32,
    pub slot_b_crc: u32,
    pub slot_a_size: u32,
    pub slot_b_size: u32,
    pub reserved: [u32; 4],
    pub table_crc: u32,
}

impl BootTable {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub const fn new_default() -> Self {
        let mut table = Self {
            magic: AGLA_BOOT_MAGIC,
            version: BOOT_TABLE_VERSION,
            sequence: 1,
            active_slot: SlotId::SlotA as u8,
            candidate_slot: 0xFF, // None
            boot_attempts: 0,
            state: BootState::Normal as u8,
            slot_a_crc: 0,
            slot_b_crc: 0,
            slot_a_size: 0,
            slot_b_size: 0,
            reserved: [0; 4],
            table_crc: 0,
        };
        table.table_crc = 0; // Filled on compute
        table
    }

    pub fn compute_crc(&self) -> u32 {
        let bytes = self.as_bytes();
        // Compute CRC excluding the last 4 bytes (the table_crc field itself)
        crc32(&bytes[..Self::SIZE - 4])
    }

    pub fn update_crc(&mut self) {
        self.table_crc = self.compute_crc();
    }

    pub fn is_valid(&self) -> bool {
        self.magic == AGLA_BOOT_MAGIC
            && self.version == BOOT_TABLE_VERSION
            && (self.active_slot == 0 || self.active_slot == 1)
            && self.table_crc == self.compute_crc()
    }

    pub fn get_active_slot(&self) -> SlotId {
        SlotId::from_u8(self.active_slot).unwrap_or(SlotId::SlotA)
    }

    pub fn get_candidate_slot(&self) -> Option<SlotId> {
        SlotId::from_u8(self.candidate_slot)
    }

    pub fn get_state(&self) -> BootState {
        BootState::from_u8(self.state)
    }

    pub fn set_candidate(&mut self, slot: SlotId, size: u32, crc: u32) {
        self.sequence = self.sequence.wrapping_add(1);
        self.candidate_slot = slot as u8;
        self.boot_attempts = 2;
        self.state = BootState::TestingCandidate as u8;
        match slot {
            SlotId::SlotA => {
                self.slot_a_size = size;
                self.slot_a_crc = crc;
            }
            SlotId::SlotB => {
                self.slot_b_size = size;
                self.slot_b_crc = crc;
            }
        }
        self.update_crc();
    }

    pub fn confirm_active(&mut self) {
        if let Some(candidate) = self.get_candidate_slot() {
            self.active_slot = candidate as u8;
        }
        self.sequence = self.sequence.wrapping_add(1);
        self.candidate_slot = 0xFF; // None
        self.boot_attempts = 0;
        self.state = BootState::Normal as u8;
        self.update_crc();
    }

    pub fn mark_rollback(&mut self) {
        self.sequence = self.sequence.wrapping_add(1);
        self.candidate_slot = 0xFF; // None
        self.boot_attempts = 0;
        self.state = BootState::RolledBack as u8;
        self.update_crc();
    }

    pub fn get_boot_clock_mhz(&self) -> u32 {
        match self.reserved[0] {
            300 => 300,
            500 => 500,
            _ => 150,
        }
    }

    pub fn set_boot_clock_mhz(&mut self, mhz: u32) {
        self.sequence = self.sequence.wrapping_add(1);
        self.reserved[0] = match mhz {
            300 => 300,
            500 => 500,
            _ => 150,
        };
        self.update_crc();
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                Self::SIZE,
            )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let table = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Self) };
        if table.is_valid() {
            Some(table)
        } else {
            None
        }
    }
}

/// Selects the current valid BootTable using ping-pong redundancy.
/// Returns `(active_table, next_write_sector_offset)`.
pub fn select_active_boot_table(sec0_bytes: &[u8], sec1_bytes: &[u8]) -> (BootTable, u32) {
    let t0 = BootTable::from_bytes(sec0_bytes);
    let t1 = BootTable::from_bytes(sec1_bytes);

    match (t0, t1) {
        (Some(a), Some(b)) => {
            if b.sequence > a.sequence {
                (b, BOOT_SECTOR_0_OFFSET) // b is newer, next write goes to sector 0
            } else {
                (a, BOOT_SECTOR_1_OFFSET) // a is newer or equal, next write goes to sector 1
            }
        }
        (Some(a), None) => (a, BOOT_SECTOR_1_OFFSET), // Only sec 0 valid, write next to sec 1
        (None, Some(b)) => (b, BOOT_SECTOR_0_OFFSET), // Only sec 1 valid, write next to sec 0
        (None, None) => {
            let mut def = BootTable::new_default();
            def.update_crc();
            (def, BOOT_SECTOR_0_OFFSET)
        }
    }
}

/// Signed Firmware Container Header (64 bytes).
#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FirmwareHeader {
    pub magic: [u8; 8],        // b"AGLA_FW\0"
    pub board_id: [u8; 8],     // e.g. b"PICO2\0\0\0" or b"BAKUPROB"
    pub version: [u8; 16],     // e.g. b"1.1.0\0..."
    pub target_slot: u8,       // 0 = Any/Dynamic, 1 = Slot A, 2 = Slot B
    pub reserved: [u8; 3],
    pub payload_size: u32,
    pub payload_crc: u32,
    pub reserved2: [u32; 4],
    pub header_crc: u32,
}

impl FirmwareHeader {
    pub const SIZE: usize = core::mem::size_of::<Self>();

    pub fn new(board_id: &[u8], version: &[u8], target_slot: u8, payload_size: u32, payload_crc: u32) -> Self {
        let mut hdr = Self {
            magic: AGLA_FW_MAGIC,
            board_id: [0; 8],
            version: [0; 16],
            target_slot,
            reserved: [0; 3],
            payload_size,
            payload_crc,
            reserved2: [0; 4],
            header_crc: 0,
        };
        let b_len = board_id.len().min(8);
        hdr.board_id[..b_len].copy_from_slice(&board_id[..b_len]);
        let v_len = version.len().min(16);
        hdr.version[..v_len].copy_from_slice(&version[..v_len]);
        hdr.header_crc = hdr.compute_crc();
        hdr
    }

    pub fn compute_crc(&self) -> u32 {
        let bytes = self.as_bytes();
        crc32(&bytes[..Self::SIZE - 4])
    }

    pub fn is_valid(&self) -> bool {
        self.magic == AGLA_FW_MAGIC && self.header_crc == self.compute_crc()
    }

    pub fn matches_board(&self, expected_board: &[u8]) -> bool {
        let len = expected_board.len().min(8);
        &self.board_id[..len] == &expected_board[..len]
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                Self::SIZE,
            )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < Self::SIZE {
            return None;
        }
        let hdr = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Self) };
        if hdr.is_valid() {
            Some(hdr)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc32() {
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }

    #[test]
    fn test_boot_table_lifecycle() {
        let mut bt = BootTable::new_default();
        bt.update_crc();
        assert!(bt.is_valid());
        assert_eq!(bt.get_active_slot(), SlotId::SlotA);
        assert_eq!(bt.get_state(), BootState::Normal);

        // Candidate staging
        bt.set_candidate(SlotId::SlotB, 400_000, 0x12345678);
        assert!(bt.is_valid());
        assert_eq!(bt.get_candidate_slot(), Some(SlotId::SlotB));
        assert_eq!(bt.get_state(), BootState::TestingCandidate);
        assert_eq!(bt.boot_attempts, 2);

        // Confirm
        bt.confirm_active();
        assert!(bt.is_valid());
        assert_eq!(bt.get_active_slot(), SlotId::SlotB);
        assert_eq!(bt.get_candidate_slot(), None);
        assert_eq!(bt.get_state(), BootState::Normal);

        // Rollback
        bt.mark_rollback();
        assert!(bt.is_valid());
        assert_eq!(bt.get_state(), BootState::RolledBack);

        // Clock configuration
        assert_eq!(bt.get_boot_clock_mhz(), 150);
        bt.set_boot_clock_mhz(300);
        assert!(bt.is_valid());
        assert_eq!(bt.get_boot_clock_mhz(), 300);
        bt.set_boot_clock_mhz(500);
        assert!(bt.is_valid());
        assert_eq!(bt.get_boot_clock_mhz(), 500);
        bt.set_boot_clock_mhz(150);
        assert!(bt.is_valid());
        assert_eq!(bt.get_boot_clock_mhz(), 150);
    }

    #[test]
    fn test_ping_pong_selection() {
        let mut t0 = BootTable::new_default();
        t0.sequence = 10;
        t0.update_crc();

        let mut t1 = BootTable::new_default();
        t1.sequence = 11;
        t1.active_slot = SlotId::SlotB as u8;
        t1.update_crc();

        let (chosen, next_off) = select_active_boot_table(t0.as_bytes(), t1.as_bytes());
        assert_eq!(chosen.sequence, 11);
        assert_eq!(chosen.get_active_slot(), SlotId::SlotB);
        assert_eq!(next_off, BOOT_SECTOR_0_OFFSET);

        // If sector 1 is corrupted
        let (chosen2, next_off2) = select_active_boot_table(t0.as_bytes(), &[0xFF; 64]);
        assert_eq!(chosen2.sequence, 10);
        assert_eq!(next_off2, BOOT_SECTOR_1_OFFSET);
    }

    #[test]
    fn test_firmware_header() {
        let hdr = FirmwareHeader::new(b"PICO2", b"1.1.0", 0, 412_000, 0xAABBCCDD);
        assert!(hdr.is_valid());
        assert!(hdr.matches_board(b"PICO2"));
        assert!(!hdr.matches_board(b"BAKUPROBE"));

        let bytes = hdr.as_bytes();
        let parsed = FirmwareHeader::from_bytes(bytes).expect("parse valid header");
        assert_eq!(parsed.payload_size, 412_000);
        assert_eq!(parsed.payload_crc, 0xAABBCCDD);
    }
}
