#![no_std]
#![no_main]

use core::arch::asm;
use cortex_m::peripheral::SCB;
use cortex_m_rt::entry;
use embassy_rp::flash::{Blocking, Flash, FLASH_BASE};
use embassy_rp::peripherals::FLASH;
use embassy_rp::watchdog::Watchdog;
use embassy_time::Duration;
use panic_halt as _;
use defmt_rtt as _;

use agentla_boot_proto::{
    select_active_boot_table, BootState, BootTable,
    BOOT_SECTOR_0_OFFSET, BOOT_SECTOR_1_OFFSET,
};

const FLASH_SIZE_BYTES: usize = 4 * 1024 * 1024;
const RAM_BASE: u32 = 0x2000_0000;
const RAM_END: u32 = 0x2008_2000; // 520 KB RAM on RP2350

fn read_flash_header(offset: u32, buf: &mut [u8; BootTable::SIZE]) {
    let ptr = (FLASH_BASE as usize + offset as usize) as *const u8;
    unsafe {
        core::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), BootTable::SIZE);
    }
}

fn write_boot_table(flash: &mut Flash<'_, FLASH, Blocking, FLASH_SIZE_BYTES>, offset: u32, table: &mut BootTable) {
    table.update_crc();
    let mut page = [0xFFu8; 256];
    page[..BootTable::SIZE].copy_from_slice(table.as_bytes());

    let _ = flash.blocking_erase(offset, offset + 4096);
    let _ = flash.blocking_write(offset, &page);
}

#[entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    let mut s0 = [0u8; BootTable::SIZE];
    let mut s1 = [0u8; BootTable::SIZE];
    read_flash_header(BOOT_SECTOR_0_OFFSET, &mut s0);
    read_flash_header(BOOT_SECTOR_1_OFFSET, &mut s1);

    let (mut active_table, next_write_offset) = select_active_boot_table(&s0, &s1);

    let mut target_slot = active_table.get_active_slot();
    let mut is_candidate_run = false;

    if active_table.get_state() == BootState::TestingCandidate {
        if let Some(candidate) = active_table.get_candidate_slot() {
            if active_table.boot_attempts > 0 {
                // Decrement attempt counter and test candidate slot
                active_table.boot_attempts -= 1;
                target_slot = candidate;
                is_candidate_run = true;

                // Commit updated attempt count to flash
                let mut flash = Flash::<_, Blocking, FLASH_SIZE_BYTES>::new_blocking(p.FLASH);
                write_boot_table(&mut flash, next_write_offset, &mut active_table);
            } else {
                // Candidate failed to confirm! Rollback
                active_table.mark_rollback();
                target_slot = active_table.get_active_slot();

                let mut flash = Flash::<_, Blocking, FLASH_SIZE_BYTES>::new_blocking(p.FLASH);
                write_boot_table(&mut flash, next_write_offset, &mut active_table);
            }
        }
    }

    let mut slot_addr = target_slot.xip_address();
    let mut sp = unsafe { *(slot_addr as *const u32) };
    let mut pc = unsafe { *((slot_addr + 4) as *const u32) };

    // Sanity check slot validity: SP must be in RAM, PC must be in slot flash
    let valid_sp = sp >= RAM_BASE && sp <= RAM_END;
    let valid_pc = pc >= slot_addr && pc < (slot_addr + 1200 * 1024);

    if (!valid_sp || !valid_pc) && is_candidate_run {
        // Fallback to active slot immediately
        target_slot = active_table.get_active_slot();
        slot_addr = target_slot.xip_address();
        sp = unsafe { *(slot_addr as *const u32) };
        pc = unsafe { *((slot_addr + 4) as *const u32) };
        is_candidate_run = false;
    }

    if is_candidate_run {
        let mut wd = Watchdog::new(p.WATCHDOG);
        wd.start(Duration::from_millis(8000));
    }

    // Set Vector Table Offset Register to selected slot
    unsafe {
        (*SCB::PTR).vtor.write(slot_addr);
        asm!(
            "msr msp, {}",
            "bx {}",
            in(reg) sp,
            in(reg) pc,
            options(noreturn)
        );
    }
}
