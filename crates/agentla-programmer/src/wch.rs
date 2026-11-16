//! WCH-Link protocol parser, command interpreter, and flash programming engine.
//!
//! Emulates WCH-Link firmware version 2.12 over RVSWD for CH32V targets.
//! Compatible with MounRiver Studio, `wlink`, WCH OpenOCD, and probe-rs.

use crate::rvdm::Rvdm;
use crate::rvswd::DMI_OK;
use crate::traits::RvswdIo;

pub const CMD_SET_WRITE_REGION: u8 = 0x01;
pub const CMD_PROGRAM: u8 = 0x02;
pub const CMD_SET_READ_REGION: u8 = 0x03;
pub const CMD_PROTECT: u8 = 0x06;
pub const CMD_DMI: u8 = 0x08;
pub const CMD_IAP_CHECK: u8 = 0x0A;
pub const CMD_RESET: u8 = 0x0B;
pub const CMD_SPEED: u8 = 0x0C;
pub const CMD_CONTROL: u8 = 0x0D;
pub const CMD_CHIP_INFO: u8 = 0x11;

pub const FW_MAJOR: u8 = 2;
pub const FW_MINOR: u8 = 12;
pub const PROBE_TYPE_CH549: u8 = 1;

pub const ESIG_FLASH_SIZE: u32 = 0x1FFFF7E0;
pub const ESIG_UID: u32 = 0x1FFFF7E8;
pub const FLASH_OBR: u32 = 0x4002201C;
pub const FLASH_OBR_RDPRT: u32 = 1 << 1;

pub const FLASH_KEYR: u32 = 0x40022004;
pub const FLASH_OBKEYR: u32 = 0x40022008;
pub const FLASH_STATR: u32 = 0x4002200C;
pub const FLASH_CTLR: u32 = 0x40022010;
pub const FLASH_ADDR: u32 = 0x40022014;
pub const FLASH_MODEKEYR: u32 = 0x40022024;

pub const KEY1: u32 = 0x45670123;
pub const KEY2: u32 = 0xCDEF89AB;

pub const CTLR_PG: u32 = 1 << 0;
pub const CTLR_MER: u32 = 1 << 2;
pub const CTLR_STRT: u32 = 1 << 6;
pub const CTLR_LOCK: u32 = 1 << 7;
pub const CTLR_FLOCK: u32 = 1 << 15;
pub const CTLR_FTPG: u32 = 1 << 16;
pub const CTLR_FTER: u32 = 1 << 17;
pub const CTLR_PGSTRT: u32 = 1 << 21;

pub const STATR_BSY: u32 = 1 << 0;
pub const STATR_WRBSY: u32 = 1 << 1;
pub const STATR_WRPRTERR: u32 = 1 << 4;
pub const STATR_EOP: u32 = 1 << 5;

pub const PAGE_SIZE: usize = 256;
pub const ACK_EVERY: u32 = 4096;

#[inline]
fn be32(p: &[u8]) -> u32 {
    ((p[0] as u32) << 24) | ((p[1] as u32) << 16) | ((p[2] as u32) << 8) | (p[3] as u32)
}

#[inline]
fn put_be32(p: &mut [u8], v: u32) {
    p[0] = (v >> 24) as u8;
    p[1] = (v >> 16) as u8;
    p[2] = (v >> 8) as u8;
    p[3] = v as u8;
}

pub fn family_of(chip_id: u32) -> u8 {
    match chip_id >> 20 {
        0x003 => 0x09, // CH32V003
        0x103 => 0x01, // CH32V103
        0x203 | 0x208 => 0x05, // CH32V20x
        0x303 | 0x305 | 0x307 | 0x317 => 0x06, // CH32V30x
        _ => 0x06,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashState {
    Idle,
    Routine,
    Writing,
}

pub struct WchLinkHandler<T: RvswdIo> {
    pub rvdm: Rvdm<T>,
    pub family: u8,
    pub chip_id: u32,
    pub wr_addr: u32,
    pub wr_len: u32,
    pub rd_addr: u32,
    pub rd_len: u32,
    pub rd_pos: u32,
    pub reading: bool,

    // Flash programming state
    pub flash_state: FlashState,
    pub routine_left: u32,
    pub flash_base: u32,
    pub flash_total: u32,
    pub flash_received: u32,
    pub flash_acked_at: u32,
    pub page_buffer: [u8; PAGE_SIZE],
    pub page_fill: usize,
    pub session_open: bool,
    pub was_running: bool,
    pub ack_due: bool,
    pub flash_failed: bool,
    pub verify: bool,
}

impl<T: RvswdIo> WchLinkHandler<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            rvdm: Rvdm::new(transport),
            family: 0x06,
            chip_id: 0,
            wr_addr: 0,
            wr_len: 0,
            rd_addr: 0,
            rd_len: 0,
            rd_pos: 0,
            reading: false,

            flash_state: FlashState::Idle,
            routine_left: 0,
            flash_base: 0,
            flash_total: 0,
            flash_received: 0,
            flash_acked_at: 0,
            page_buffer: [0; PAGE_SIZE],
            page_fill: 0,
            session_open: false,
            was_running: false,
            ack_due: false,
            flash_failed: false,
            verify: false,
        }
    }

    fn reply(&self, resp: &mut [u8], cmd: u8, payload: &[u8]) -> usize {
        resp[0] = 0x82;
        resp[1] = cmd;
        resp[2] = payload.len() as u8;
        resp[3..3 + payload.len()].copy_from_slice(payload);
        3 + payload.len()
    }

    fn reply1(&self, resp: &mut [u8], cmd: u8, v: u8) -> usize {
        self.reply(resp, cmd, &[v])
    }

    fn unknown(&self, resp: &mut [u8]) -> usize {
        self.reply1(resp, 0x81, 0xFF)
    }

    fn open_flash_session(&mut self) -> bool {
        if self.session_open {
            return true;
        }
        if let Ok(was_run) = self.rvdm.begin() {
            self.was_running = was_run;
            self.session_open = true;
            let _ = self.rvdm.mem_write32(FLASH_KEYR, KEY1);
            let _ = self.rvdm.mem_write32(FLASH_KEYR, KEY2);
            let _ = self.rvdm.mem_write32(FLASH_MODEKEYR, KEY1);
            let _ = self.rvdm.mem_write32(FLASH_MODEKEYR, KEY2);
            if let Ok(ctrl) = self.rvdm.mem_read32(FLASH_CTLR) {
                return (ctrl & CTLR_LOCK) == 0;
            }
        }
        false
    }

    fn close_flash_session(&mut self, stay_halted: bool) {
        if !self.session_open {
            return;
        }
        let _ = self.rvdm.mem_write32(FLASH_CTLR, CTLR_LOCK | CTLR_FLOCK);
        let _ = self.rvdm.end(self.was_running && !stay_halted);
        self.session_open = false;
    }

    fn wait_flash_idle(&mut self) -> bool {
        for _ in 0..2000 {
            if let Ok(s) = self.rvdm.mem_read32(FLASH_STATR) {
                if (s & STATR_BSY) == 0 {
                    if (s & (STATR_EOP | STATR_WRPRTERR)) != 0 {
                        let _ = self.rvdm.mem_write32(FLASH_STATR, STATR_EOP | STATR_WRPRTERR);
                    }
                    return (s & STATR_WRPRTERR) == 0;
                }
            }
            crate::delay::delay_cycles(1500);
        }
        false
    }

    fn write_flash_page(&mut self, addr: u32, page: &[u8]) -> bool {
        // Fast page erase
        if self.rvdm.mem_write32(FLASH_CTLR, CTLR_FTER).is_err()
            || self.rvdm.mem_write32(FLASH_ADDR, addr).is_err()
            || self.rvdm.mem_write32(FLASH_CTLR, CTLR_FTER | CTLR_STRT).is_err()
            || !self.wait_flash_idle()
            || self.rvdm.mem_write32(FLASH_CTLR, 0).is_err()
        {
            return false;
        }

        // Fast page program via program buffer streaming
        if self.rvdm.mem_write32(FLASH_CTLR, CTLR_FTPG).is_err()
            || self.rvdm.mem_stream32_bytes(addr, page, FLASH_STATR, STATR_WRBSY).is_err()
            || self.rvdm.mem_write32(FLASH_CTLR, CTLR_FTPG | CTLR_PGSTRT).is_err()
            || !self.wait_flash_idle()
        {
            let _ = self.rvdm.mem_write32(FLASH_CTLR, 0);
            return false;
        }
        let _ = self.rvdm.mem_write32(FLASH_CTLR, 0);
        true
    }

    fn verify_flash_page(&mut self, addr: u32, page: &[u8]) -> bool {
        for i in 0..PAGE_SIZE / 4 {
            let offset = (i * 4) as u32;
            let val = match self.rvdm.mem_read32(addr + offset) {
                Ok(v) => v,
                Err(_) => return false,
            };
            let expected = u32::from_le_bytes([
                page[i * 4],
                page[i * 4 + 1],
                page[i * 4 + 2],
                page[i * 4 + 3],
            ]);
            if val != expected {
                return false;
            }
        }
        true
    }

    pub fn mass_erase(&mut self) -> bool {
        let ok = self.open_flash_session()
            && self.rvdm.mem_write32(FLASH_CTLR, CTLR_MER).is_ok()
            && self.rvdm.mem_write32(FLASH_CTLR, CTLR_MER | CTLR_STRT).is_ok()
            && self.wait_flash_idle()
            && self.rvdm.mem_write32(FLASH_CTLR, 0).is_ok();
        self.close_flash_session(true);
        ok
    }

    /// Execute incoming command request `0x81, CMD, LEN, payload...`.
    pub fn execute(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        if req.len() < 3 || req[0] != 0x81 {
            return self.unknown(resp);
        }
        let cmd = req[1];
        let payload = &req[3..];
        let n = payload.len();

        match cmd {
            CMD_CONTROL => {
                if n < 1 {
                    return self.unknown(resp);
                }
                match payload[0] {
                    0x01 => {
                        // Firmware version 2.12
                        let p = [FW_MAJOR, FW_MINOR, PROBE_TYPE_CH549, 0];
                        self.reply(resp, CMD_CONTROL, &p)
                    }
                    0x02 => {
                        // Attach
                        if let Ok(id) = self.rvdm.attach() {
                            self.chip_id = id;
                            self.family = family_of(id);
                            let mut p = [0u8; 5];
                            p[0] = self.family;
                            put_be32(&mut p[1..5], id);
                            self.reply(resp, CMD_CONTROL, &p)
                        } else {
                            // Cannot connect
                            resp[0] = 0x81;
                            resp[1] = 0x55;
                            resp[2] = 0x01;
                            resp[3] = 0x01;
                            4
                        }
                    }
                    0x04 => self.reply1(resp, CMD_CONTROL, 0x02),
                    0x17 => self.reply1(resp, CMD_CONTROL, 0x05),
                    0x06 => self.reply1(resp, CMD_CONTROL, 0x00),
                    sub @ (0x03 | 0x05 | 0x07 | 0x10 | 0x18 | 0xFF) => {
                        self.reply1(resp, CMD_CONTROL, sub)
                    }
                    _ => self.reply1(resp, CMD_CONTROL, 0xFF),
                }
            }

            CMD_SPEED => {
                if n < 2 {
                    return self.unknown(resp);
                }
                self.family = payload[0];
                let speed_tier = payload[1];
                let freq_khz = crate::clock::wch_speed_to_khz(speed_tier);
                let _ = self.rvdm.io.set_clock(freq_khz);
                self.reply1(resp, CMD_SPEED, 0x01)
            }

            CMD_DMI => {
                if n < 6 {
                    return self.unknown(resp);
                }
                let addr = payload[0];
                let mut val = be32(&payload[1..5]);
                let op = payload[5];
                let mut status = DMI_OK;

                match op {
                    1 => match self.rvdm.dmi_read(addr) {
                        Ok(v) => val = v,
                        Err(_) => status = 2,
                    },
                    2 => match self.rvdm.dmi_write(addr, val) {
                        Ok(()) => {}
                        Err(_) => status = 2,
                    },
                    _ => val = 0,
                }
                let mut out = [0u8; 6];
                out[0] = addr;
                put_be32(&mut out[1..5], val);
                out[5] = status;
                self.reply(resp, CMD_DMI, &out)
            }

            CMD_RESET => {
                if n < 1 {
                    return self.unknown(resp);
                }
                let halt = payload[0] == 0x02;
                let _ = self.rvdm.reset(halt);
                self.reply1(resp, CMD_RESET, payload[0])
            }

            CMD_CHIP_INFO => {
                // Sent by wlink and MounRiver. Returns 20 bytes with NO header!
                let mut flash_size = 0xFFFFFFFFu32;
                let mut uid0 = 0xFFFFFFFFu32;
                let mut uid1 = 0xFFFFFFFFu32;
                let mut uid2 = 0xFFFFFFFFu32;
                if let Ok(was_run) = self.rvdm.begin() {
                    flash_size = self.rvdm.mem_read32(ESIG_FLASH_SIZE).unwrap_or(0xFFFFFFFF);
                    uid0 = self.rvdm.mem_read32(ESIG_UID).unwrap_or(0xFFFFFFFF);
                    uid1 = self.rvdm.mem_read32(ESIG_UID + 4).unwrap_or(0xFFFFFFFF);
                    uid2 = self.rvdm.mem_read32(ESIG_UID + 8).unwrap_or(0xFFFFFFFF);
                    let _ = self.rvdm.end(was_run);
                }
                put_be32(&mut resp[0..4], flash_size);
                put_be32(&mut resp[4..8], uid0);
                put_be32(&mut resp[8..12], uid1);
                put_be32(&mut resp[12..16], uid2);
                put_be32(&mut resp[16..20], self.chip_id);
                20
            }

            CMD_PROTECT => {
                if n < 1 {
                    return self.unknown(resp);
                }
                match payload[0] {
                    0x01 => self.reply1(resp, CMD_PROTECT, 0x02), // Unprotected
                    0x04 => self.reply1(resp, CMD_PROTECT, 0x00), // Unprotected (newer form)
                    0x02 => self.reply1(resp, CMD_PROTECT, 0x02), // Unprotect
                    0x03 => self.reply1(resp, CMD_PROTECT, 0x03), // Protect
                    _ => self.unknown(resp),
                }
            }

            CMD_SET_WRITE_REGION => {
                if n >= 8 {
                    self.wr_addr = be32(&payload[0..4]);
                    self.wr_len = be32(&payload[4..8]);
                }
                self.reply1(resp, CMD_SET_WRITE_REGION, 0x01)
            }

            CMD_SET_READ_REGION => {
                if n < 8 {
                    return self.unknown(resp);
                }
                self.rd_addr = be32(&payload[0..4]);
                self.rd_len = be32(&payload[4..8]);
                self.reply1(resp, CMD_SET_READ_REGION, 0x01)
            }

            CMD_PROGRAM => {
                if n < 1 {
                    return self.unknown(resp);
                }
                let sub = payload[0];
                match sub {
                    0x01 => {
                        self.mass_erase();
                    }
                    0x05 => {
                        self.flash_state = FlashState::Routine;
                        self.routine_left = 512;
                    }
                    0x02 | 0x07 => {
                        self.start_flash_write(false);
                    }
                    0x04 => {
                        self.start_flash_write(true);
                    }
                    0x08 => {
                        self.flash_state = FlashState::Idle;
                        self.close_flash_session(false);
                    }
                    0x0C => {
                        self.rd_pos = 0;
                        self.reading = self.rd_len != 0;
                    }
                    0x06 => {}
                    _ => return self.unknown(resp),
                }
                self.reply1(resp, CMD_PROGRAM, sub)
            }

            CMD_IAP_CHECK => self.reply1(resp, CMD_IAP_CHECK, 0x01),

            _ => self.unknown(resp),
        }
    }

    fn start_flash_write(&mut self, verify: bool) {
        self.verify = verify;
        self.flash_base = self.wr_addr & !((PAGE_SIZE as u32) - 1);
        self.flash_total = ((self.wr_addr - self.flash_base) + self.wr_len + (PAGE_SIZE as u32) - 1)
            & !((PAGE_SIZE as u32) - 1);
        self.flash_received = 0;
        self.flash_acked_at = 0;
        self.page_fill = 0;
        self.ack_due = false;
        self.flash_failed = false;
        self.flash_state = if self.flash_total > 0 {
            FlashState::Writing
        } else {
            FlashState::Idle
        };
        if self.flash_state == FlashState::Writing && !self.open_flash_session() {
            self.flash_failed = true;
        }
    }

    /// Whether the device wants OUT data on EP 0x02.
    pub fn wants_data(&self) -> bool {
        self.flash_state == FlashState::Routine
            || (self.flash_state == FlashState::Writing && !self.ack_due)
    }

    /// Process OUT data received on EP 0x02.
    pub fn data_out(&mut self, buf: &[u8]) {
        if self.flash_state == FlashState::Routine {
            let len = buf.len() as u32;
            self.routine_left = self.routine_left.saturating_sub(len);
            if self.routine_left == 0 {
                self.flash_state = FlashState::Idle;
            }
            return;
        }

        if self.flash_state != FlashState::Writing {
            return;
        }

        let mut remaining = buf;
        while !remaining.is_empty() {
            let n = (PAGE_SIZE - self.page_fill).min(remaining.len());
            self.page_buffer[self.page_fill..self.page_fill + n].copy_from_slice(&remaining[..n]);
            self.page_fill += n;
            self.flash_received += n as u32;
            remaining = &remaining[n..];

            if self.page_fill == PAGE_SIZE {
                let page_addr = self.flash_base + self.flash_received - (PAGE_SIZE as u32);
                if !self.flash_failed {
                    let page_clone = self.page_buffer;
                    if !self.write_flash_page(page_addr, &page_clone)
                        || (self.verify && !self.verify_flash_page(page_addr, &page_clone))
                    {
                        self.flash_failed = true;
                        self.close_flash_session(false);
                    }
                }
                self.page_fill = 0;
            }
        }

        if !self.flash_failed
            && (self.flash_received - self.flash_acked_at >= ACK_EVERY
                || self.flash_received >= self.flash_total)
        {
            self.flash_acked_at = self.flash_received;
            self.ack_due = true;
        }
    }

    /// Check if flow-control ACK (`41 01 01 04`) is pending for EP 0x82.
    pub fn ack_pending(&self, ack: &mut [u8; 4]) -> bool {
        if self.ack_due {
            ack[0] = 0x41;
            ack[1] = 0x01;
            ack[2] = 0x01;
            ack[3] = 0x04;
            true
        } else {
            false
        }
    }

    /// Notify that flow-control ACK was sent.
    pub fn ack_sent(&mut self) {
        self.ack_due = false;
        if self.flash_state == FlashState::Writing && self.flash_received >= self.flash_total {
            self.flash_state = FlashState::Idle;
            self.close_flash_session(false);
        }
    }

    /// Read next chunk for EP 0x82 memory read.
    pub fn read_chunk(&mut self, buf: &mut [u8]) -> usize {
        if !self.reading || self.rd_pos >= self.rd_len {
            self.reading = false;
            return 0;
        }
        let n = ((self.rd_len - self.rd_pos) as usize).min(buf.len());
        let was_run = match self.rvdm.begin() {
            Ok(r) => r,
            Err(_) => {
                self.reading = false;
                return 0;
            }
        };

        for i in (0..n).step_by(4) {
            let addr = self.rd_addr + self.rd_pos + (i as u32);
            let w = self.rvdm.mem_read32(addr).unwrap_or(0);
            let chunk_len = (n - i).min(4);
            let bytes = w.to_le_bytes();
            buf[i..i + chunk_len].copy_from_slice(&bytes[..chunk_len]);
        }
        let _ = self.rvdm.end(was_run);
        self.rd_pos += n as u32;
        if self.rd_pos >= self.rd_len {
            self.reading = false;
        }
        n
    }
}
