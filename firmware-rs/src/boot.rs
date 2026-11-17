use agentla_boot_proto::{
    select_active_boot_table, BootState, BootTable, SlotId,
    BOOT_SECTOR_0_OFFSET, BOOT_SECTOR_1_OFFSET, BOOT_SECTOR_SIZE,
};
use embassy_rp::flash::FLASH_BASE;

/// Zero-copy read of 64 bytes from XIP flash space.
/// Build marker emitted by build.rs: git hash, link slot, dirty flag, UTC stamp.
pub mod build_info {
    include!(concat!(env!("OUT_DIR"), "/build_marker.rs"));
}

/// Raw contents of both boot-table ping-pong sectors, for diagnostics.
pub fn dump_boot_sectors() -> ([u8; BootTable::SIZE], [u8; BootTable::SIZE]) {
    let mut s0 = [0u8; BootTable::SIZE];
    let mut s1 = [0u8; BootTable::SIZE];
    read_sector_header(BOOT_SECTOR_0_OFFSET, &mut s0);
    read_sector_header(BOOT_SECTOR_1_OFFSET, &mut s1);
    (s0, s1)
}

fn read_sector_header(offset: u32, buf: &mut [u8; BootTable::SIZE]) {
    let ptr = (FLASH_BASE as usize + offset as usize) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), BootTable::SIZE);
    }
}

/// Reads the current active BootTable and the target offset for the next write.
pub fn get_boot_state() -> (BootTable, u32) {
    let mut s0 = [0u8; BootTable::SIZE];
    let mut s1 = [0u8; BootTable::SIZE];
    read_sector_header(BOOT_SECTOR_0_OFFSET, &mut s0);
    read_sector_header(BOOT_SECTOR_1_OFFSET, &mut s1);
    select_active_boot_table(&s0, &s1)
}

/// Detects the currently executing slot based on Vector Table Offset Register (VTOR) or PC.
///
/// Slot A XIP range is `0x1000_4000..0x1010_4000`, Slot B is
/// `0x1010_4000..0x1020_4000`. The previous check only tested `>= Slot B`,
/// which is fine for VTOR (a Slot A VTOR is below it) but the PC fallback needs
/// an explicit upper bound so a Slot A PC is never read as Slot B.
pub fn current_executing_slot() -> SlotId {
    let vtor = unsafe { (*cortex_m::peripheral::SCB::PTR).vtor.read() };
    let pc: u32;
    unsafe {
        core::arch::asm!("mov {}, pc", out(reg) pc);
    }
    const SLOT_A_XIP: u32 = 0x1000_4000;
    const SLOT_B_XIP: u32 = 0x1010_4000;
    const SLOT_END: u32 = 0x1020_4000;

    let vtor_is_slot_b = vtor >= SLOT_B_XIP && vtor < SLOT_END;
    let pc_is_slot_b = pc >= SLOT_B_XIP && pc < SLOT_END;
    let vtor_is_slot_a = vtor >= SLOT_A_XIP && vtor < SLOT_B_XIP;
    let pc_is_slot_a = pc >= SLOT_A_XIP && pc < SLOT_B_XIP;

    if vtor_is_slot_b || (!vtor_is_slot_a && pc_is_slot_b) {
        SlotId::SlotB
    } else if vtor_is_slot_a || pc_is_slot_a {
        SlotId::SlotA
    } else {
        // Unknown VTOR (e.g. before the bootloader sets it): trust the PC.
        if pc >= SLOT_B_XIP {
            SlotId::SlotB
        } else {
            SlotId::SlotA
        }
    }
}

/// Writes an updated BootTable to the inactive ping-pong sector.
pub fn commit_boot_table(table: &mut BootTable) -> Result<(), ()> {
    let (_active, next_offset) = get_boot_state();
    table.update_crc();

    let mut page_buf = [0xFFu8; 256];
    page_buf[..BootTable::SIZE].copy_from_slice(table.as_bytes());

    crate::storage::flash_boot_sector_commit(next_offset, &page_buf)
}

/// Stages an uploaded firmware in the given slot as the candidate boot target.
pub fn stage_candidate(slot: SlotId, size: u32, crc: u32) -> Result<(), ()> {
    let (mut table, _) = get_boot_state();
    table.set_candidate(slot, size, crc);
    commit_boot_table(&mut table)
}

/// Confirms the currently executing candidate slot as permanently active.
pub fn ota_confirm() -> Result<(), ()> {
    let (mut table, _) = get_boot_state();
    if table.get_state() == BootState::TestingCandidate {
        defmt::info!("OTA: Confirming candidate slot as permanently active.");
        table.confirm_active();
        commit_boot_table(&mut table)?;
    } else {
        defmt::info!("OTA: Running in normal mode; confirm no-op.");
    }
    Ok(())
}

/// Initiates an immediate rollback to the alternate slot.
pub fn ota_rollback() -> Result<(), ()> {
    let (mut table, _) = get_boot_state();
    defmt::warn!("OTA: Triggering manual rollback to alternate slot.");
    let target = table.get_active_slot().other();
    table.active_slot = target as u8;
    table.mark_rollback();
    commit_boot_table(&mut table)
}
