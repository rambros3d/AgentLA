#![allow(dead_code)] // Save/boot/delete surface is consumed by the REST router (task 4.2);
                     // payload-pointer helper for group 4/5 too.

//! # Hardware Profile Manager (flash-resident, C-format parity)
//!
//! 32 × 2 KiB slots at offset `0x180000` (64 KiB window below the LittleFS partition),
//! records binary-compatible with the C firmware's `profile_slot_t` so offline tooling
//! (`.uf2`-dump readers) can parse the slot table:
//!
//! | offset | size | field            | value                                   |
//! |-------:|-----:|------------------|-----------------------------------------|
//! | 0      | 4    | magic (u32 LE)   | `0x41474C41` ("AGLA")                   |
//! | 4      | 2    | version (u16 LE) | 1                                       |
//! | 6      | 2    | flags (u16 LE)   | IN_USE=1, BOOT_DEFAULT=2, READ_ONLY=4   |
//! | 8      | 32   | name             | NUL-padded / truncated to 31            |
//! | 40     | 4    | payload_crc32    | reflected CRC32, poly `0xEDB88320`      |
//! | 44     | 4    | payload_len (u32)|                                         |
//! | 48     | 2000 | payload          | CRC covers the first `payload_len` B    |
//!
//! Sector granularity is 4 KiB (2 slots/sector): save/delete rewrite the parent sector by
//! read-modify-erase-program under [`PROFILE_MUTEX`] (design decision 10; the accepted
//! power-loss window is the C-firmware parity behaviour).
//!
//! Slot 0 is the virtual read-only **Safe State**, synthesized at read time, never stored.
//!
//! Boot flow (task 2.4): [`scan_boot_default`] runs before the LittleFS mount, is entirely
//! XIP (zero-copy reads) and **never erases** — erase-in-boot is banned (design decision 5).
//!
//! Apply semantics (`hardware-profile-manager` delta spec): quiesce all flexible pins,
//! settle 5 ms, record the slot as active. Payloads are **never parsed** — profiles stay
//! metadata records (C parity) until a payload schema exists.

use core::sync::atomic::{AtomicU8, Ordering};

use embassy_rp::flash::FLASH_BASE;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

use crate::storage::{
    profile_flash_erase, profile_flash_read, profile_flash_write, PROFILE_WINDOW_START,
};

// ---------------------------------------------------------------------------
// Format constants
// ---------------------------------------------------------------------------

pub const SLOT_SIZE: u32 = 2048;
pub const SECTOR_SIZE: u32 = 4096; // = 2 slots
pub const SLOT_COUNT: u32 = 32;
pub const PAYLOAD_MAX: u32 = 2000;
pub const MAGIC: u32 = 0x4147_4C41; // "AGLA"
pub const VERSION: u16 = 1;
pub const FLAG_IN_USE: u16 = 1 << 0;
pub const FLAG_BOOT_DEFAULT: u16 = 1 << 1;
pub const FLAG_READ_ONLY: u16 = 1 << 2;

const HEADER_LEN: usize = 48;
const OFF_MAGIC: usize = 0;
const OFF_VERSION: usize = 4;
const OFF_FLAGS: usize = 6;
const OFF_NAME: usize = 8;
const OFF_CRC: usize = 40;
const OFF_LEN: usize = 44;
const OFF_PAYLOAD: usize = 48;

const _: () = assert!(SLOT_SIZE as usize == 48 + PAYLOAD_MAX as usize); // packed, no padding
const _: () = assert!(2 * SLOT_SIZE == SECTOR_SIZE);

pub const METADATA_PAYLOAD: &[u8] = b"{}"; // C parity: payload stays metadata-only
pub const SAFE_STATE_NAME: &str = "Safe State (Hi-Z)";
/// C `profile_manager.c` safe-state payload byte-for-byte (synthesized, never stored).
pub const SAFE_STATE_PAYLOAD: &[u8] = b"{\"name\":\"Safe State (Hi-Z)\",\"safe\":true,\"power\":{\"vtarget_enabled\":false,\"voltage_v\":0.0},\"pins\":[],\"instruments\":{}}";
pub const SAFE_STATE_CRC: u32 = record::crc32(SAFE_STATE_PAYLOAD);

// ---------------------------------------------------------------------------
// Pure record codec (host-testable; no flash access here)
// ---------------------------------------------------------------------------

pub mod record {
    //! Slot-record codec over byte buffers (LE, C-layout). Pure logic — unit tests live
    //! at the bottom of this file; remaining on-target verification is task 5.3.

    use super::{
        HEADER_LEN, MAGIC, OFF_CRC, OFF_FLAGS, OFF_LEN, OFF_MAGIC, OFF_NAME,
        OFF_PAYLOAD, OFF_VERSION, PAYLOAD_MAX, SLOT_SIZE, VERSION,
    };

    /// Standard CRC32: reflected polynomial `0xEDB88320`, init/xor-out `0xFFFFFFFF`
    /// (C `calculate_crc32` parity; ~15-line software implementation, no crate needed).
    pub const fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        let mut i = 0;
        while i < data.len() {
            crc ^= data[i] as u32;
            let mut bit = 0;
            while bit < 8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                bit += 1;
            }
            i += 1;
        }
        crc ^ 0xFFFF_FFFF
    }

    /// Parsed slot header (payload bytes are not copied; `payload_len`/`payload_crc32`
    /// are carried for integrity checks and REST reporting).
    #[derive(Debug, Clone, PartialEq, Eq, defmt::Format)]
    pub struct Header {
        pub flags: u16,
        /// ASCII-safe printable name (raw flash bytes ≥ 0x80 are dropped on decode).
        pub name: heapless::String<32>,
        pub payload_len: u32,
        pub payload_crc32: u32,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
    pub enum DecodeErr {
        /// Every header byte erased — blank slot.
        Blank,
        /// Wrong magic/version, out-of-range length, or CRC mismatch.
        Corrupt,
    }

    /// Encode a record into `record` (a slot-sized slice, e.g. one half of the sector
    /// buffer). Name is truncated to 31 bytes (C `strncpy` parity), payload asserted ≤
    /// [`PAYLOAD_MAX`].
    pub fn encode(record: &mut [u8], flags: u16, name: &str, payload: &[u8]) {
        debug_assert!(record.len() >= SLOT_SIZE as usize);
        debug_assert!(payload.len() as u32 <= PAYLOAD_MAX);
        record[..SLOT_SIZE as usize].fill(0);
        record[OFF_MAGIC..OFF_MAGIC + 4].copy_from_slice(&MAGIC.to_le_bytes());
        record[OFF_VERSION..OFF_VERSION + 2].copy_from_slice(&VERSION.to_le_bytes());
        record[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&flags.to_le_bytes());
        let bytes = name.as_bytes();
        let n = bytes.len().min(31);
        record[OFF_NAME..OFF_NAME + n].copy_from_slice(&bytes[..n]);
        let crc = crc32(payload);
        record[OFF_CRC..OFF_CRC + 4].copy_from_slice(&crc.to_le_bytes());
        record[OFF_LEN..OFF_LEN + 4].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        record[OFF_PAYLOAD..OFF_PAYLOAD + payload.len()].copy_from_slice(payload);
    }

    /// Parse + validate a record: C `profile_get_slot` semantics (magic, version,
    /// `payload_len ≤ 2000`, CRC over the payload). `hdr` is the 48 header bytes,
    /// `payload` the (zero-copy XIP or buffer) payload region.
    pub fn decode(hdr: &[u8], payload: &[u8]) -> Result<Header, DecodeErr> {
        if hdr[..HEADER_LEN].iter().all(|&b| b == 0xFF) {
            return Err(DecodeErr::Blank);
        }
        let magic = u32::from_le_bytes(hdr[OFF_MAGIC..OFF_MAGIC + 4].try_into().unwrap());
        if magic != MAGIC {
            return Err(DecodeErr::Corrupt);
        }
        let version = u16::from_le_bytes(hdr[OFF_VERSION..OFF_VERSION + 2].try_into().unwrap());
        if version != VERSION {
            return Err(DecodeErr::Corrupt);
        }
        let flags = u16::from_le_bytes(hdr[OFF_FLAGS..OFF_FLAGS + 2].try_into().unwrap());
        let payload_len = u32::from_le_bytes(hdr[OFF_LEN..OFF_LEN + 4].try_into().unwrap());
        if payload_len > PAYLOAD_MAX {
            return Err(DecodeErr::Corrupt);
        }
        let stored_crc = u32::from_le_bytes(hdr[OFF_CRC..OFF_CRC + 4].try_into().unwrap());
        let len = payload_len as usize;
        if len > payload.len() || crc32(&payload[..min(len, payload.len())]) != stored_crc {
            return Err(DecodeErr::Corrupt);
        }
        let mut name = heapless::String::new();
        for &b in &hdr[OFF_NAME..OFF_NAME + 32] {
            if b == 0 {
                break;
            }
            // Keep ASCII printable only: heapless::String is UTF-8; REST names are ASCII
            // (C names written through the router are as well).
            if (0x20u8..0x7f).contains(&b) && name.push(b as char).is_err() {
                break;
            }
        }
        Ok(Header {
            flags,
            name,
            payload_len,
            payload_crc32: stored_crc,
        })
    }

    /// Flags u16 of a header, without validation (boot scans / sector flag twiddling).
    pub const fn peek_flags(hdr: &[u8]) -> u16 {
        u16::from_le_bytes([hdr[OFF_FLAGS], hdr[OFF_FLAGS + 1]])
    }

    /// Peek magic of a header (distinguishes untouched halves before flag edits).
    pub const fn peek_magic(hdr: &[u8]) -> u32 {
        u32::from_le_bytes([
            hdr[OFF_MAGIC],
            hdr[OFF_MAGIC + 1],
            hdr[OFF_MAGIC + 2],
            hdr[OFF_MAGIC + 3],
        ])
    }

    /// Read-modify the flags of one header in the sector buffer: dry/clear mask bits on
    /// the u16. Returns true when the record had a plausible magic (edit was permitted).
    pub fn edit_flags(hdr: &mut [u8], set_mask: u16, clear_mask: u16) -> bool {
        if peek_magic(hdr) != MAGIC {
            return false;
        }
        let flags = peek_flags(hdr) | set_mask & !clear_mask;
        hdr[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&flags.to_le_bytes());
        true
    }

    const fn min(a: usize, b: usize) -> usize {
        if a < b { a } else { b }
    }
}

// ---------------------------------------------------------------------------
// Slot enumerations / headers
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, defmt::Format)]
pub enum SlotRead {
    /// Virtual read-only Safe State (slot 0; never physically stored).
    SafeState(record::Header),
    /// Valid, CRC-checked record.
    InUse(record::Header),
    /// Fully erased — no record.
    Empty,
    /// Wrong magic/version/length or payload CRC mismatch (REST: "corrupted slot").
    Corrupt,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
pub enum ProfileError {
    /// Slot outside `1..32` for a mutating operation.
    OutOfRange,
    /// Slot 0 is the virtual read-only Safe State.
    ReadOnly,
    /// Operated on a slot with no valid record (REST maps this to 404 "slot not in use").
    NotInUse,
    /// Record failed its integrity checks.
    Corrupt,
    /// Payload length above [`PAYLOAD_MAX`].
    PayloadTooLarge,
    /// Storage-level flash error.
    Io,
}

/// XIP base of a slot's record (zero-copy reads; the slot table never overlaps the
/// LittleFS partition, which lives above `PROFILE_WINDOW_START + 0x10000`).
fn slot_offset(slot: u32) -> u32 {
    debug_assert!(slot < SLOT_COUNT);
    PROFILE_WINDOW_START + slot * SLOT_SIZE
}

/// Read/validate one slot. Slot 0 is synthesized ([`SlotRead::SafeState`]); slots beyond
/// [`SLOT_COUNT`] read as `Empty`. Only 48 stack bytes are touched; the payload CRC is
/// computed straight from XIP memory.
pub fn read_slot(slot: u32) -> SlotRead {
    if slot == 0 {
        return SlotRead::SafeState(record::Header {
            flags: FLAG_IN_USE | FLAG_READ_ONLY,
            name: safe_state_name(),
            payload_len: SAFE_STATE_PAYLOAD.len() as u32,
            payload_crc32: SAFE_STATE_CRC,
        });
    }
    if slot >= SLOT_COUNT {
        return SlotRead::Empty;
    }
    let mut hdr = [0u8; HEADER_LEN];
    profile_flash_read(slot_offset(slot), &mut hdr);
    match record::decode(&hdr, xip_payload(slot)) {
        Ok(header) => SlotRead::InUse(header),
        Err(record::DecodeErr::Blank) => SlotRead::Empty,
        Err(_) => SlotRead::Corrupt,
    }
}

/// Zero-copy XIP slice over one slot's full payload region (2000 bytes). The borrow ends
/// before any flash op (`PROFILE_MUTEX` scopes below), so no torn read is observable.
fn xip_payload(slot: u32) -> &'static [u8] {
    unsafe {
        core::slice::from_raw_parts(
            (FLASH_BASE as usize + (slot_offset(slot) + OFFSET_BASE) as usize) as *const u8,
            PAYLOAD_MAX as usize,
        )
    }
}

const OFFSET_BASE: u32 = 48;

fn safe_state_name() -> heapless::String<32> {
    let mut name = heapless::String::new();
    let _ = name.push_str(SAFE_STATE_NAME);
    name
}

// ---------------------------------------------------------------------------
// Active-slot tracking (metadata only — payloads never parsed, C parity)
// ---------------------------------------------------------------------------

static ACTIVE_SLOT: AtomicU8 = AtomicU8::new(0);
static ACTIVE_NAME: Mutex<CriticalSectionRawMutex, heapless::String<32>> = Mutex::new(heapless::String::new());

/// Currently active slot id; `0` = Safe State.
pub fn active_slot() -> u32 {
    ACTIVE_SLOT.load(Ordering::Relaxed) as u32
}

/// Name of the active profile (bounced copy; callers may inspect without holding state).
pub fn active_name() -> heapless::String<32> {
    ACTIVE_NAME.try_lock().expect("active name lock free").clone()
}

pub fn safe_state_active() -> bool {
    active_slot() == 0
}

fn set_active(slot: u32, name: &str) {
    debug_assert!(slot < SLOT_COUNT);
    ACTIVE_SLOT.store(slot as u8, Ordering::Relaxed);
    let mut cell = ACTIVE_NAME
        .try_lock()
        .expect("active name lock free (sync, no awaits)");
    cell.clear();
    let _ = cell.push_str(name);
}

// ---------------------------------------------------------------------------
// Boot-default scan (task 2.4 — read-only XIP; NEVER erases at boot)
// ---------------------------------------------------------------------------

/// Scan slots `1..32` for `IN_USE && BOOT_DEFAULT` with C-`profile_manager_init`
/// semantics (first hit wins; record validated via magic + CRC). On a hit, that slot is
/// recorded active; otherwise the virtual Safe State stays active (initial state).
/// Returns the activated slot, if any.
pub fn scan_boot_default() -> Option<u32> {
    let mut found = None;
    for slot in 1..SLOT_COUNT as u8 {
        if let SlotRead::InUse(header) = read_slot(slot as u32) {
            if header.flags & (FLAG_BOOT_DEFAULT | FLAG_IN_USE) == (FLAG_BOOT_DEFAULT | FLAG_IN_USE)
            {
                set_active(slot as u32, header.name.as_str());
                found = Some(slot as u32);
                break;
            }
        }
    }
    match found {
        Some(slot) => defmt::info!("Boot default: profile slot {} active", slot),
        None => defmt::info!("No boot-default profile; Safe State (Hi-Z) active"),
    }
    found
}

// ---------------------------------------------------------------------------
// Mutating slot operations (all under PROFILE_MUTEX: read-modify-erase-program
// sequences must not interleave; LittleFS lives above the window and on FS_MUTEX)
// ---------------------------------------------------------------------------

/// Scratch buffer for sector-level read-modify-write, kept behind [`PROFILE_MUTEX`] so
/// saves/deletes never burn 4 KiB of task stack.
pub struct ProfileHal {
    sector_buf: [u8; SECTOR_SIZE as usize],
}

static PROFILE_MUTEX: Mutex<CriticalSectionRawMutex, ProfileHal> = Mutex::new(ProfileHal {
    sector_buf: [0u8; SECTOR_SIZE as usize],
});

/// Base offset (absolute flash address) of the 4 KiB parent sector of `slot`.
fn sector_offset(slot: u32) -> u32 {
    PROFILE_WINDOW_START + (slot & !1) * SLOT_SIZE
}

/// Byte offset of the slot's 2048 B half inside its parent sector.
fn half_offset(slot: u32) -> usize {
    (slot & 1) as usize * SLOT_SIZE as usize
}

/// Save a profile slot (C `profile_save_slot` parity): the record carries
/// [`FLAG_IN_USE`] (plus [`FLAG_BOOT_DEFAULT`] when `is_boot`), name, payload and its
/// CRC. When `is_boot`, [`FLAG_BOOT_DEFAULT`] is cleared on every other slot so exactly
/// one boot designation survives.
pub fn save_profile(
    slot: u32,
    name: &str,
    is_boot: bool,
    payload: &[u8],
) -> Result<(), ProfileError> {
    if slot == 0 {
        return Err(ProfileError::ReadOnly);
    }
    if slot >= SLOT_COUNT {
        return Err(ProfileError::OutOfRange);
    }
    if payload.len() as u32 > PAYLOAD_MAX {
        return Err(ProfileError::PayloadTooLarge);
    }
    let flags = if is_boot {
        FLAG_IN_USE | FLAG_BOOT_DEFAULT
    } else {
        FLAG_IN_USE
    };

    let mut guard = PROFILE_MUTEX.try_lock().expect("profile mutex free");
    let sector = sector_offset(slot);
    let half = half_offset(slot);

    // READ sector (XIP) → MUTATE the slot's half.
    profile_flash_read(sector, &mut guard.sector_buf);
    let rec = &mut guard.sector_buf[half..half + SLOT_SIZE as usize];
    record::encode(rec, flags, name, payload);
    // Designating a new boot slot also clears a previous BOOT_DEFAULT of the neighbouring
    // slot in this sector, so the erase+program pass below fixes sector-internal state too.
    if is_boot {
        let neighbour_base = half ^ SLOT_SIZE as usize;
        let neighbour = &mut guard.sector_buf[neighbour_base..neighbour_base + HEADER_LEN as usize];
        if neighbour_base != 0 {
            record::edit_flags(neighbour, 0, FLAG_BOOT_DEFAULT);
        }
    }

    // ERASE sector → PROGRAM sector (page-granular bursts).
    profile_flash_erase(sector, sector + SECTOR_SIZE);
    profile_flash_write(sector, &guard.sector_buf);

    defmt::info!("Profile slot {} saved (boot={})", slot, is_boot);

    if is_boot {
        clear_boot_default_flags(Some(slot), &mut guard);
    }
    Ok(())
}

/// Designate the boot-default slot (C `profile_set_boot_default` parity). `slot == 0`
/// clears the designation everywhere (power cycles boot into Safe State). Requires the
/// target slot to hold a valid record.
pub fn set_boot_default(slot: u32) -> Result<(), ProfileError> {
    if slot >= SLOT_COUNT {
        return Err(ProfileError::OutOfRange);
    }
    if slot == 0 {
        // Clear across all slots so the system boots into Safe State.
        let mut guard = PROFILE_MUTEX.try_lock().expect("profile mutex free");
        clear_boot_default_flags(None, &mut guard);
        defmt::info!("Boot default cleared (Safe State active at boot)");
        return Ok(());
    }
    match read_slot(slot) {
        SlotRead::InUse(_) => {}
        SlotRead::Empty => return Err(ProfileError::NotInUse),
        SlotRead::Corrupt => return Err(ProfileError::Corrupt),
        SlotRead::SafeState(_) => unreachable!(),
    }

    let mut guard = PROFILE_MUTEX.try_lock().expect("profile mutex free");
    let sector = sector_offset(slot);
    let half = half_offset(slot);
    profile_flash_read(sector, &mut guard.sector_buf);
    let target_base = half;
    let neighbour_base = half ^ SLOT_SIZE as usize;
    // Set BOOT_DEFAULT on the target; clear it on the neighbour (same sector half-pass):
    let target = &mut guard.sector_buf[target_base..target_base + HEADER_LEN];
    let _ = record::edit_flags(target, FLAG_BOOT_DEFAULT, 0);
    if slot != 1 {
        // Neighbour half edit — but never touch the (virtual) slot-0 half of sector 0.
        let neighbour = &mut guard.sector_buf[neighbour_base..neighbour_base + HEADER_LEN];
        record::edit_flags(neighbour, 0, FLAG_BOOT_DEFAULT);
    }
    profile_flash_erase(sector, sector + SECTOR_SIZE);
    profile_flash_write(sector, &guard.sector_buf);

    clear_boot_default_flags(Some(slot), &mut guard);
    defmt::info!("Boot default set to slot {}", slot);
    Ok(())
}

/// Delete (blank) a slot's record half (C `profile_delete_slot` parity). If the deleted
/// slot is currently active, the system transitions to Safe State (quiesce + active reset).
pub fn delete_profile(slot: u32) -> Result<(), ProfileError> {
    if slot == 0 {
        return Err(ProfileError::ReadOnly);
    }
    if slot >= SLOT_COUNT {
        return Err(ProfileError::OutOfRange);
    }
    match read_slot(slot) {
        SlotRead::InUse(_) => {}
        SlotRead::Empty => return Err(ProfileError::NotInUse),
        SlotRead::Corrupt => return Err(ProfileError::Corrupt),
        SlotRead::SafeState(_) => unreachable!(),
    }

    let mut guard = PROFILE_MUTEX.try_lock().expect("profile mutex free");
    let sector = sector_offset(slot);
    let half = half_offset(slot);
    profile_flash_read(sector, &mut guard.sector_buf);
    guard.sector_buf[half..half + SLOT_SIZE as usize].fill(0xFF);
    profile_flash_erase(sector, sector + SECTOR_SIZE);
    profile_flash_write(sector, &guard.sector_buf);
    drop(guard);

    defmt::info!("Profile slot {} deleted", slot);
    if active_slot() == slot {
        crate::pin_owner::quiesce_and_reset();
        set_active(0, SAFE_STATE_NAME);
        defmt::info!("Active profile deleted; Safe State active");
    }
    Ok(())
}

/// What [`apply_slot`] validated and recorded.
pub struct AppliedProfile {
    pub slot: u32,
    pub name: heapless::String<32>,
}

/// Apply a profile (C `profile_apply_slot` parity + hardware-profile-manager delta spec):
/// quiesce every flexible pin to high-Z SIO inputs via the pin registry, settle 5 ms
/// (rail discharge), then record the slot as active. Payload contents are never
/// interpreted. Slot 0 applies the Safe State (no settle delay in C either — the settle
/// happens on the quiesce path; we keep the same ordering as C: quiesce → report).
pub async fn apply_slot(slot: u32) -> Result<AppliedProfile, ProfileError> {
    if slot >= SLOT_COUNT {
        return Err(ProfileError::OutOfRange);
    }
    let header = match read_slot(slot) {
        SlotRead::SafeState(header) => header,
        SlotRead::InUse(header) => header,
        SlotRead::Empty => return Err(ProfileError::NotInUse),
        SlotRead::Corrupt => return Err(ProfileError::Corrupt),
    };

    // 1. Safe intermediate tri-state (aborts every active flexible claim).
    crate::pin_owner::quiesce_and_reset();
    // 2. Settle: discharge rails and settle bus signals.
    if slot != 0 {
        embassy_time::Timer::after(embassy_time::Duration::from_millis(5)).await;
    }
    // 3. Transition active state (metadata only; pins stay quiesced — C parity).
    set_active(slot, header.name.as_str());
    defmt::info!("Profile slot {} applied ({})", slot, header.name.as_str());
    Ok(AppliedProfile {
        slot,
        name: header.name,
    })
}

/// Read-modify-erase-program every sector that still carries `FLAG_BOOT_DEFAULT`
/// (excluding `keep`'s slot). Requires the caller to hold [`PROFILE_MUTEX`] and reuses
/// its [`ProfileHal::sector_buf`] scratch buffer.
fn clear_boot_default_flags(keep: Option<u32>, guard: &mut ProfileHal) {
    for sector in 0..SLOT_COUNT / 2 {
        let slots = [sector * 2, sector * 2 + 1];
        // Fast path: nothing to do when neither candidate slot carries the flag.
        let mut has_flag = false;
        for &slot in &slots {
            if slot == 0 || Some(slot) == keep {
                continue;
            }
            let mut hdr = [0u8; HEADER_LEN];
            profile_flash_read(slot_offset(slot), &mut hdr);
            if record::peek_magic(&hdr) == MAGIC && record::peek_flags(&hdr) & FLAG_BOOT_DEFAULT != 0
            {
                has_flag = true;
            }
        }
        if !has_flag {
            continue;
        }
        // Rewrite this sector with the flag cleared.
        let base = sector_offset(slots[1]);
        profile_flash_read(base, &mut guard.sector_buf);
        for &slot in &slots {
            if slot == 0 || Some(slot) == keep {
                continue;
            }
            let hdr = &mut guard.sector_buf[half_offset(slot)..half_offset(slot) + HEADER_LEN];
            record::edit_flags(hdr, 0, FLAG_BOOT_DEFAULT);
        }
        profile_flash_erase(base, base + SECTOR_SIZE);
        profile_flash_write(base, &guard.sector_buf);
    }
}

// ---------------------------------------------------------------------------
// Unit tests (host-only; see note in pin_owner.rs — the on-target record behaviour
// regression runs under the REST hardware round, task 5.3).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::record::{crc32, decode, encode, peek_flags, DecodeErr};
    use super::{FLAG_BOOT_DEFAULT, FLAG_IN_USE, FLAG_READ_ONLY, METADATA_PAYLOAD};

    /// Canonical CRC32 reference vector: CRC32("123456789") = 0xCBF43926.
    #[test]
    fn crc32_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0x0000_0000);
    }

    #[test]
    fn encode_decode_round_trip() {
        let mut rec = [0u8; super::SLOT_SIZE as usize];
        encode(
            &mut rec,
            FLAG_IN_USE | FLAG_BOOT_DEFAULT,
            "MyProfile",
            METADATA_PAYLOAD,
        );
        let hdr = &rec[..48];
        assert_eq!(peek_flags(hdr), FLAG_IN_USE | FLAG_BOOT_DEFAULT);
        let header = decode(hdr, &rec[48..]).unwrap();
        assert_eq!(header.name.as_str(), "MyProfile");
        assert_eq!(header.payload_len, 2);
        assert_eq!(header.payload_crc32, crc32(METADATA_PAYLOAD));
    }

    #[test]
    fn blank_detected() {
        let rec = [0xFFu8; super::SLOT_SIZE as usize];
        assert_eq!(decode(&rec[..48], &rec[48..]), Err(DecodeErr::Blank));
    }

    #[test]
    fn corrupt_crc_detected() {
        let mut rec = [0u8; super::SLOT_SIZE as usize];
        encode(&mut rec, FLAG_IN_USE, "X", b"payload");
        rec[48] = !rec[48]; // break payload
        assert_eq!(decode(&rec[..48], &rec[48..]), Err(DecodeErr::Corrupt));
    }

    #[test]
    fn out_of_range_payload_len_detected() {
        let mut rec = [0u8; super::SLOT_SIZE as usize];
        encode(&mut rec, FLAG_IN_USE, "X", b"p");
        // Forge payload_len = 2001 (> 2000).
        rec[44..48].copy_from_slice(&2001u32.to_le_bytes());
        assert_eq!(decode(&rec[..48], &rec[48..]), Err(DecodeErr::Corrupt));
    }

    #[test]
    fn name_truncated_to_31_bytes() {
        let mut rec = [0u8; super::SLOT_SIZE as usize];
        encode(&mut rec, FLAG_IN_USE, "123456789012345678901234567890123456789", b"x");
        let header = decode(&rec[..48], &rec[48..]).unwrap();
        assert_eq!(header.name.len(), 31);
        assert_eq!(header.name.as_str().len(), 31);
    }

    #[test]
    fn flag_twiddle_helpers() {
        let mut rec = [0xFFu8; super::SLOT_SIZE as usize];
        encode(&mut rec, FLAG_IN_USE, "A", b"{");

        let hdr = &mut rec[..48];
        // Setting flags works on a valid magic:
        assert!(super::record::edit_flags(&mut hdr[..], FLAG_BOOT_DEFAULT, 0));
        assert_eq!(peek_flags(hdr), FLAG_IN_USE | FLAG_BOOT_DEFAULT);
        assert!(super::record::edit_flags(&mut hdr[..], 0, FLAG_BOOT_DEFAULT));
        assert_eq!(peek_flags(hdr), FLAG_IN_USE);
        // Blank slots (no magic) refuse edits:
        let mut blank = [0xFFu8; 48];
        assert!(!super::record::edit_flags(&mut blank, FLAG_READ_ONLY, 0));

        // Virtual safe-state record CRC is const-compilable and matches its payload.
        assert_eq!(super::SAFE_STATE_CRC, crc32(super::SAFE_STATE_PAYLOAD));
    }
}
