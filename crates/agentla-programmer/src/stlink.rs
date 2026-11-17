//! ST-Link/V2-1 protocol parser, command interpreter, and SWD execution engine.
//!
//! Compatible with STM32CubeIDE, STM32CubeProgrammer, OpenOCD, pyOCD, and stlink tools.
//! Reports version `V2J48M35` (`0x2C23`) under USB VID `0x0483` / PID `0x3752`.

use crate::delay::delay_cycles;
use crate::swd::{
    Adiv5Access, Adiv5Engine, ADIV5_AP_CSW_DEFAULT, AP_CSW, AP_DRW, AP_TAR, DP_DPIDR,
};
use crate::swo::{SwoEngine, SWO_DEFAULT_BAUD};
use crate::traits::SwdIo;

pub const STLINK_OK: u8 = 0x80;
pub const STLINK_FALSE: u8 = 0x81;
pub const STLINK_CORE_RUNNING: u8 = 0x80;
pub const STLINK_CORE_HALTED: u8 = 0x81;
pub const STLINK_NO_DEVICE: u8 = 0x05;
pub const STLINK_SWD_AP_WAIT: u8 = 0x10;
pub const STLINK_SWD_AP_FAULT: u8 = 0x11;
pub const STLINK_SWD_AP_ERROR: u8 = 0x12;
pub const STLINK_SWD_AP_PARITY: u8 = 0x13;
pub const STLINK_SWD_DP_WAIT: u8 = 0x14;
pub const STLINK_SWD_DP_FAULT: u8 = 0x15;
pub const STLINK_SWD_DP_ERROR: u8 = 0x16;
pub const STLINK_SWD_DP_PARITY: u8 = 0x17;

pub const STLINK_GET_VERSION: u8 = 0xF1;
pub const STLINK_DEBUG_COMMAND: u8 = 0xF2;
pub const STLINK_DFU_COMMAND: u8 = 0xF3;
pub const STLINK_SWIM_COMMAND: u8 = 0xF4;
pub const STLINK_GET_CURRENT_MODE: u8 = 0xF5;
pub const STLINK_GET_TARGET_VOLTAGE: u8 = 0xF7;
pub const STLINK_BOOTLOADER_COMMAND: u8 = 0xF9;

pub const STLINK_MODE_DFU: u8 = 0x00;
pub const STLINK_MODE_MASS: u8 = 0x01;
pub const STLINK_MODE_DEBUG: u8 = 0x02;

pub const STLINK_DFU_EXIT: u8 = 0x07;

// Debug subcommands
pub const DBG_GETSTATUS: u8 = 0x01;
pub const DBG_FORCEDEBUG: u8 = 0x02;
pub const DBG_APIV1_RESETSYS: u8 = 0x03;
pub const DBG_APIV1_READALLREGS: u8 = 0x04;
pub const DBG_APIV1_READREG: u8 = 0x05;
pub const DBG_APIV1_WRITEREG: u8 = 0x06;
pub const DBG_READMEM_32BIT: u8 = 0x07;
pub const DBG_WRITEMEM_32BIT: u8 = 0x08;
pub const DBG_RUNCORE: u8 = 0x09;
pub const DBG_STEPCORE: u8 = 0x0A;
pub const DBG_APIV1_SETFP: u8 = 0x0B;
pub const DBG_READMEM_8BIT: u8 = 0x0C;
pub const DBG_WRITEMEM_8BIT: u8 = 0x0D;
pub const DBG_APIV1_CLEARFP: u8 = 0x0E;
pub const DBG_APIV1_WRITEDEBUGREG: u8 = 0x0F;
pub const DBG_APIV1_SETWATCHPOINT: u8 = 0x10;
pub const DBG_APIV1_ENTER: u8 = 0x20;
pub const DBG_EXIT: u8 = 0x21;
pub const DBG_READCOREID: u8 = 0x22;
pub const DBG_APIV2_ENTER: u8 = 0x30;
pub const DBG_APIV2_READ_IDCODES: u8 = 0x31;
pub const DBG_APIV2_RESETSYS: u8 = 0x32;
pub const DBG_APIV2_READREG: u8 = 0x33;
pub const DBG_APIV2_WRITEREG: u8 = 0x34;
pub const DBG_APIV2_WRITEDEBUGREG: u8 = 0x35;
pub const DBG_APIV2_READDEBUGREG: u8 = 0x36;
pub const DBG_APIV2_SETWATCHPOINT: u8 = 0x37;
pub const DBG_APIV2_SETFP: u8 = 0x38;
pub const DBG_APIV2_CLEARFP: u8 = 0x39;
pub const DBG_APIV2_READALLREGS: u8 = 0x3A;
pub const DBG_APIV2_GETLASTRWSTATUS: u8 = 0x3B;
pub const DBG_APIV2_DRIVE_NRST: u8 = 0x3C;
pub const DBG_APIV2_READALLREGS_FP: u8 = 0x3D;
pub const DBG_APIV2_GETLASTRWSTATUS2: u8 = 0x3E;
pub const DBG_APIV2_START_TRACE_RX: u8 = 0x40;
pub const DBG_APIV2_STOP_TRACE_RX: u8 = 0x41;
pub const DBG_APIV2_GET_TRACE_NB: u8 = 0x42;
pub const DBG_APIV2_SWD_SET_FREQ: u8 = 0x43;
pub const DBG_APIV2_JTAG_SET_FREQ: u8 = 0x44;
pub const DBG_APIV2_READ_DAP_REG: u8 = 0x45;
pub const DBG_APIV2_WRITE_DAP_REG: u8 = 0x46;
pub const DBG_APIV2_READMEM_16BIT: u8 = 0x47;
pub const DBG_APIV2_WRITEMEM_16BIT: u8 = 0x48;
pub const DBG_APIV2_INIT_AP: u8 = 0x4B;
pub const DBG_APIV2_CLOSE_AP_DBG: u8 = 0x4C;
pub const DBG_WRITEMEM_32BIT_NOINC: u8 = 0x50;
pub const DBG_APIV2_RW_MISC_OUT: u8 = 0x51;
pub const DBG_APIV2_RW_MISC_IN: u8 = 0x52;
pub const DBG_READMEM_32BIT_NOINC: u8 = 0x54;
pub const DBG_APIV2_BOARD_INFO: u8 = 0x56;
pub const DBG_APIV2_GET_CAPS: u8 = 0x57;
pub const DBG_APIV2_GET_TRACE_CAPS: u8 = 0x58;

pub const DBG_ENTER_SWD: u8 = 0xA3;

// Cortex-M Core Debug Registers
pub const DHCSR: u32 = 0xE000EDF0;
pub const DCRSR: u32 = 0xE000EDF4;
pub const DCRDR: u32 = 0xE000EDF8;
pub const AIRCR: u32 = 0xE000ED0C;
pub const MVFR0: u32 = 0xE000EF40;

pub const DBGKEY: u32 = 0xA05F0000;
pub const C_DEBUGEN: u32 = 1 << 0;
pub const C_HALT: u32 = 1 << 1;
pub const C_STEP: u32 = 1 << 2;
pub const S_REGRDY: u32 = 1 << 16;
pub const S_HALT: u32 = 1 << 17;

pub const STLINK_USB_VID: u16 = 0x0483;
pub const STLINK_USB_PID: u16 = 0x3752;
pub const STLINK_VER_REPORTED: u16 = (2 << 12) | (48 << 6) | 35; // V2J48M35 = 0x2C23

fn write_u16_ascii(mut val: u16, buf: &mut [u8]) -> usize {
    if buf.is_empty() {
        return 0;
    }
    if val == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 5];
    let mut tmp_len = 0;
    while val > 0 && tmp_len < tmp.len() {
        tmp[tmp_len] = b'0' + (val % 10) as u8;
        val /= 10;
        tmp_len += 1;
    }
    let write_len = tmp_len.min(buf.len());
    for i in 0..write_len {
        buf[i] = tmp[tmp_len - 1 - i];
    }
    write_len
}

/// Format a 16-bit ST-Link version word ((v << 12) | (j << 6) | m) into "V<v>J<j>M<m>".
/// Returns a string slice referencing the provided output buffer.
pub fn format_version(v: u16, buf: &mut [u8]) -> &str {
    let major = (v >> 12) & 0x0F;
    let jtag = (v >> 6) & 0x3F;
    let msd = v & 0x3F;

    let mut idx = 0;
    if idx < buf.len() {
        buf[idx] = b'V';
        idx += 1;
    }
    idx += write_u16_ascii(major, &mut buf[idx..]);
    if idx < buf.len() {
        buf[idx] = b'J';
        idx += 1;
    }
    idx += write_u16_ascii(jtag, &mut buf[idx..]);
    if idx < buf.len() {
        buf[idx] = b'M';
        idx += 1;
    }
    idx += write_u16_ascii(msd, &mut buf[idx..]);

    core::str::from_utf8(&buf[..idx]).unwrap_or("")
}

/// Parse an ST-Link version string ("V<v>J<j>M<m>" or "<v>J<j>M<m>") into a 16-bit word.
/// Returns None if the string is malformed or component ranges exceed bit boundaries.
pub fn parse_version(s: &str) -> Option<u16> {
    let bytes = s.trim().as_bytes();
    if bytes.is_empty() {
        return None;
    }

    let mut pos = 0;
    if bytes[pos] == b'V' || bytes[pos] == b'v' {
        pos += 1;
    }

    // Parse V
    let v_start = pos;
    let mut v: u16 = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        v = v.checked_mul(10)?.checked_add((bytes[pos] - b'0') as u16)?;
        pos += 1;
    }
    if pos == v_start || pos >= bytes.len() {
        return None;
    }

    // Must be 'J' or 'j'
    if bytes[pos] != b'J' && bytes[pos] != b'j' {
        return None;
    }
    pos += 1;

    // Parse J
    let j_start = pos;
    let mut j: u16 = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        j = j.checked_mul(10)?.checked_add((bytes[pos] - b'0') as u16)?;
        pos += 1;
    }
    if pos == j_start || pos >= bytes.len() {
        return None;
    }

    // Must be 'M' or 'm'
    if bytes[pos] != b'M' && bytes[pos] != b'm' {
        return None;
    }
    pos += 1;

    // Parse M
    let m_start = pos;
    let mut m: u16 = 0;
    while pos < bytes.len() && bytes[pos].is_ascii_digit() {
        m = m.checked_mul(10)?.checked_add((bytes[pos] - b'0') as u16)?;
        pos += 1;
    }
    if pos == m_start || pos != bytes.len() {
        return None;
    }

    // Range checks: V: 4-bit (<=15), J: 6-bit (>0 and <=63), M: 6-bit (<=63)
    if v > 15 || j == 0 || j > 63 || m > 63 {
        return None;
    }

    Some((v << 12) | (j << 6) | m)
}

// Batched RW_MISC operations (OpenOCD)
pub const MISC_ADDRESS: u8 = 1;
pub const MISC_WRITE: u8 = 2;
pub const MISC_READ: u8 = 3;
pub const MISC_APNUM: u8 = 5;
pub const MISC_MAX: u32 = 256;

#[inline]
fn get16(p: &[u8]) -> u16 {
    (p[0] as u16) | ((p[1] as u16) << 8)
}

#[inline]
fn get32(p: &[u8]) -> u32 {
    (p[0] as u32) | ((p[1] as u32) << 8) | ((p[2] as u32) << 16) | ((p[3] as u32) << 24)
}

#[inline]
fn put32(p: &mut [u8], v: u32) {
    p[0] = v as u8;
    p[1] = (v >> 8) as u8;
    p[2] = (v >> 16) as u8;
    p[3] = (v >> 24) as u8;
}

static DEFAULT_SWO_ENGINE: SwoEngine = SwoEngine::new();

pub struct StLinkHandler<T: SwdIo> {
    pub engine: Adiv5Engine<T>,
    pub mode: u8,
    pub last_status: u8,
    pub idcode: u32,
    pub current_ap: u8,
    /// DP power state. The ST-Link protocol has no explicit power-up command,
    /// so the first target access must power the DP lazily.
    pub dp_powered: bool,
    pub reported_version: u16,
    pub voltage_provider: Option<fn() -> u32>,
    misc_buf: [u8; 1024],
    misc_len: usize,
    misc_items: u32,
    pub swo: &'static SwoEngine,
}

impl<T: SwdIo> StLinkHandler<T> {
    pub const fn new(transport: T) -> Self {
        Self::with_swo(transport, &DEFAULT_SWO_ENGINE)
    }

    pub const fn with_swo(transport: T, swo: &'static SwoEngine) -> Self {
        Self {
            engine: Adiv5Engine::new(transport),
            mode: STLINK_MODE_MASS,
            last_status: STLINK_OK,
            idcode: 0,
            current_ap: 0,
            dp_powered: false,
            reported_version: STLINK_VER_REPORTED,
            voltage_provider: None,
            misc_buf: [0; 1024],
            misc_len: 0,
            misc_items: 0,
            swo,
        }
    }

    /// Returns the currently reported ST-Link 16-bit version word.
    pub fn reported_version(&self) -> u16 {
        self.reported_version
    }

    /// Sets the ST-Link 16-bit version word reported on command 0xF1.
    pub fn set_reported_version(&mut self, version: u16) {
        self.reported_version = version;
    }

    /// Sets the target voltage provider callback (returning millivolts).
    pub fn set_voltage_provider(&mut self, provider: fn() -> u32) {
        self.voltage_provider = Some(provider);
    }

    pub fn mem_read32(&mut self, addr: u32, val: &mut u32) -> u8 {
        self.mem_access(addr, None, Some(val))
    }

    /// Routed MEM-AP write. See `mem_access` for the AP selection rule.
    pub fn mem_write32(&mut self, addr: u32, val: u32) -> u8 {
        self.mem_access(addr, Some(val), None)
    }

    /// Single routed MEM-AP access honouring the ARMv8-M dual-AP layout.
    ///
    /// On STM32H5 the core private peripheral bus (DHCSR, DEMCR, ...) lives
    /// behind AP 1 (AHB-AP), not AP 0 (APB-AP). Selecting AP 0 there returns
    /// garbage, so a fixed-CSW access can never halt or inspect the core.
    ///
    /// An AHB-AP read is posted: the first DRW read returns the previous
    /// address's value, the second returns this address's data. `out` receives
    /// the second read.
    fn mem_access(&mut self, addr: u32, write: Option<u32>, out: Option<&mut u32>) -> u8 {
        let apsel = if (0xE000_0000..=0xE00F_FFFF).contains(&addr) {
            1
        } else {
            self.current_ap
        };
        let csw = if apsel == 1 { 0x23000012 } else { ADIV5_AP_CSW_DEFAULT };
        stlink_diag::step(1); // entered
        if !self.dp_powered {
            // After ENTER_SWD's line reset the DP needs a short settle before
            // it accepts power-up writes. Retry once with a delay between; a
            // genuine target can take a few ms to wake its debug domain.
            let mut powered = self.engine.power_up_debug().is_ok();
            if !powered {
                crate::delay::delay_cycles(150_000);
                powered = self.engine.power_up_debug().is_ok();
            }
            if powered {
                self.dp_powered = true;
            } else {
                stlink_diag::fail(14);
                return STLINK_SWD_AP_FAULT;
            }
        }
        stlink_diag::step(15); // powered

        if self.engine.select_ap(apsel, 0).is_err() {
            stlink_diag::fail(2);
            return STLINK_SWD_AP_FAULT;
        }
        stlink_diag::step(3); // selected
        if self.engine.write_ap(AP_CSW, csw).is_err() {
            stlink_diag::fail(4);
            return STLINK_SWD_AP_FAULT;
        }
        stlink_diag::step(5); // csw
        if self.engine.write_ap(AP_TAR, addr).is_err() {
            stlink_diag::fail(6);
            return STLINK_SWD_AP_FAULT;
        }
        stlink_diag::step(7); // tar

        let st = match write {
            Some(v) => {
                if self.engine.write_ap(AP_DRW, v).is_err() {
                    stlink_diag::fail(8);
                    return STLINK_SWD_AP_FAULT;
                }
                stlink_diag::step(9);
                STLINK_OK
            }
            None => {
                let mut dummy = 0u32;
                if self.engine.read_ap(AP_DRW, &mut dummy).is_err() {
                    stlink_diag::fail(10);
                    return STLINK_SWD_AP_FAULT;
                }
                stlink_diag::step(11);
                if out.is_none() {
                    return STLINK_OK;
                }
                let mut v = 0u32;
                if self.engine.read_ap(AP_DRW, &mut v).is_err() {
                    stlink_diag::fail(12);
                    return STLINK_SWD_AP_FAULT;
                }
                stlink_diag::step(13);
                if let Some(slot) = out {
                    *slot = v;
                }
                STLINK_OK
            }
        };
        stlink_diag::ok(st, addr);
        st
    }

    pub fn core_read_reg(&mut self, regsel: u8, val: &mut u32) -> u8 {
        let st = self.mem_write32(DCRSR, regsel as u32);
        if st != STLINK_OK {
            return st;
        }
        for _ in 0..100 {
            let mut dhcsr = 0u32;
            let st = self.mem_read32(DHCSR, &mut dhcsr);
            if st != STLINK_OK {
                return st;
            }
            if (dhcsr & S_REGRDY) != 0 {
                return self.mem_read32(DCRDR, val);
            }
            delay_cycles(150);
        }
        STLINK_FALSE
    }

    pub fn core_write_reg(&mut self, regsel: u8, val: u32) -> u8 {
        let mut st = self.mem_write32(DCRDR, val);
        if st == STLINK_OK {
            st = self.mem_write32(DCRSR, (1 << 16) | (regsel as u32));
        }
        if st != STLINK_OK {
            return st;
        }
        for _ in 0..100 {
            let mut dhcsr = 0u32;
            st = self.mem_read32(DHCSR, &mut dhcsr);
            if st == STLINK_OK && (dhcsr & S_REGRDY) != 0 {
                return STLINK_OK;
            }
            delay_cycles(150);
        }
        if st == STLINK_OK {
            STLINK_FALSE
        } else {
            st
        }
    }

    pub fn core_dhcsr(&mut self, bits: u32) -> u8 {
        self.mem_write32(DHCSR, DBGKEY | bits)
    }

    /// Access size of a memory command in bytes (1, 2, or 4), or 0 if not a memory command.
    pub fn mem_size(&self, cmd: &[u8], write: &mut bool, inc: &mut bool) -> usize {
        *inc = true;
        *write = false;
        if cmd.is_empty() || cmd[0] != STLINK_DEBUG_COMMAND || cmd.len() < 2 {
            return 0;
        }
        match cmd[1] {
            DBG_WRITEMEM_32BIT => {
                *write = true;
                4
            }
            DBG_READMEM_32BIT => 4,
            DBG_WRITEMEM_32BIT_NOINC => {
                *write = true;
                *inc = false;
                4
            }
            DBG_READMEM_32BIT_NOINC => {
                *inc = false;
                4
            }
            DBG_APIV2_WRITEMEM_16BIT => {
                *write = true;
                2
            }
            DBG_APIV2_READMEM_16BIT => 2,
            DBG_WRITEMEM_8BIT => {
                *write = true;
                1
            }
            DBG_READMEM_8BIT => 1,
            _ => 0,
        }
    }

    /// Length in bytes of the OUT data phase that must follow this command.
    pub fn out_len(&mut self, cmd: &[u8]) -> usize {
        if cmd.is_empty() {
            return 0;
        }
        if cmd.len() >= 2 && cmd[0] == STLINK_DEBUG_COMMAND && cmd[1] == DBG_APIV2_RW_MISC_OUT {
            if cmd.len() >= 6 {
                let items = get32(&cmd[2..6]);
                let len = (((items + 3) & !3) + 4 * items) as usize;
                return len;
            }
            return 0;
        }
        let mut wr = false;
        let mut inc = false;
        let sz = self.mem_size(cmd, &mut wr, &mut inc);
        if sz > 0 && wr && cmd.len() >= 8 {
            get16(&cmd[6..8]) as usize
        } else {
            0
        }
    }

    /// Length in bytes of the IN reply data phase expected for this command.
    pub fn in_len(&self, cmd: &[u8]) -> usize {
        if cmd.is_empty() {
            return 0;
        }
        let mut wr = false;
        let mut inc = false;
        let sz = self.mem_size(cmd, &mut wr, &mut inc);
        if sz == 0 || wr || cmd.len() < 8 {
            return 0;
        }
        let len = get16(&cmd[6..8]) as usize;
        if sz == 1 && len == 1 {
            2
        } else {
            len
        }
    }

    /// Receive OUT data following a memory write or RW_MISC_OUT command.
    pub fn mem_out(&mut self, cmd: &[u8], offset: usize, buf: &[u8]) {
        if cmd.len() >= 2 && cmd[0] == STLINK_DEBUG_COMMAND && cmd[1] == DBG_APIV2_RW_MISC_OUT {
            let items = get32(&cmd[2..6]);
            let total_len = (((items + 3) & !3) + 4 * items) as usize;
            if offset == 0 && items > 0 && items <= MISC_MAX && buf.len() <= self.misc_buf.len() {
                let copy_len = buf.len().min(total_len).min(self.misc_buf.len());
                self.misc_buf[..copy_len].copy_from_slice(&buf[..copy_len]);
                self.misc_len = copy_len;
                self.misc_items = items;
            }
            return;
        }

        let mut wr = false;
        let mut inc = false;
        let sz = self.mem_size(cmd, &mut wr, &mut inc);
        if sz == 0 || !wr || cmd.len() < 8 {
            return;
        }

        let base_addr = get32(&cmd[2..6]);
        let total_len = get16(&cmd[6..8]) as usize;
        let addr = base_addr + if inc { offset as u32 } else { 0 };

        if offset == 0 {
            self.last_status = STLINK_OK;
        }

        if sz == 4 {
            let n_words = buf.len() / 4;
            for i in 0..n_words {
                let word = get32(&buf[i * 4..(i + 1) * 4]);
                let a = addr + if inc { (i * 4) as u32 } else { 0 };
                let st = self.mem_write32(a, word);
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
            }
        } else if sz == 2 {
            let n_halfwords = buf.len() / 2;
            for i in 0..n_halfwords {
                let hword = get16(&buf[i * 2..(i + 1) * 2]);
                let a = addr + if inc { (i * 2) as u32 } else { 0 };
                let shift = (a & 2) * 8;
                let val = (hword as u32) << shift;
                let _ = self.engine.select_ap(self.current_ap, 0);
                let _ = self.engine.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT | 0x01);
                let _ = self.engine.write_ap(AP_TAR, a);
                let st = if self.engine.write_ap(AP_DRW, val).is_ok() {
                    STLINK_OK
                } else {
                    STLINK_SWD_AP_FAULT
                };
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
            }
        } else if sz == 1 {
            for (i, &b) in buf.iter().enumerate() {
                let a = addr + if inc { i as u32 } else { 0 };
                let shift = (a & 3) * 8;
                let val = (b as u32) << shift;
                let _ = self.engine.select_ap(self.current_ap, 0);
                let _ = self.engine.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT | 0x00);
                let _ = self.engine.write_ap(AP_TAR, a);
                let st = if self.engine.write_ap(AP_DRW, val).is_ok() {
                    STLINK_OK
                } else {
                    STLINK_SWD_AP_FAULT
                };
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
            }
        }
        let _ = total_len;
    }

    /// Stream IN data for a memory read command.
    pub fn mem_in(&mut self, cmd: &[u8], offset: usize, buf: &mut [u8]) {
        let mut wr = false;
        let mut inc = false;
        let sz = self.mem_size(cmd, &mut wr, &mut inc);
        if sz == 0 || wr || cmd.len() < 8 {
            buf.fill(0);
            return;
        }

        let base_addr = get32(&cmd[2..6]);
        let total_len = get16(&cmd[6..8]) as usize;
        let addr = base_addr + if inc { offset as u32 } else { 0 };

        if offset == 0 {
            self.last_status = STLINK_OK;
        }

        if sz == 4 {
            let n_words = buf.len() / 4;
            for i in 0..n_words {
                let a = addr + if inc { (i * 4) as u32 } else { 0 };
                let mut word = 0u32;
                let st = self.mem_read32(a, &mut word);
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
                put32(&mut buf[i * 4..(i + 1) * 4], word);
            }
        } else if sz == 2 {
            let n_halfwords = buf.len() / 2;
            for i in 0..n_halfwords {
                let a = addr + if inc { (i * 2) as u32 } else { 0 };
                let _ = self.engine.select_ap(self.current_ap, 0);
                let _ = self.engine.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT | 0x01);
                let _ = self.engine.write_ap(AP_TAR, a);
                let mut dummy = 0u32;
                let _ = self.engine.read_ap(AP_DRW, &mut dummy);
                let mut raw = 0u32;
                let st = if self.engine.read_ap(AP_DRW, &mut raw).is_ok() {
                    STLINK_OK
                } else {
                    STLINK_SWD_AP_FAULT
                };
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
                let shift = (a & 2) * 8;
                let hword = (raw >> shift) as u16;
                buf[i * 2] = hword as u8;
                buf[i * 2 + 1] = (hword >> 8) as u8;
            }
        } else if sz == 1 {
            for i in 0..buf.len().min(total_len) {
                let a = addr + if inc { i as u32 } else { 0 };
                let _ = self.engine.select_ap(self.current_ap, 0);
                let _ = self.engine.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT | 0x00);
                let _ = self.engine.write_ap(AP_TAR, a);
                let mut dummy = 0u32;
                let _ = self.engine.read_ap(AP_DRW, &mut dummy);
                let mut raw = 0u32;
                let st = if self.engine.read_ap(AP_DRW, &mut raw).is_ok() {
                    STLINK_OK
                } else {
                    STLINK_SWD_AP_FAULT
                };
                if st != STLINK_OK {
                    self.last_status = st;
                    return;
                }
                let shift = (a & 3) * 8;
                buf[i] = (raw >> shift) as u8;
            }
        }
    }

    /// Execute batched RW_MISC operations (OpenOCD).
    fn run_misc(&mut self, resp: &mut [u8]) -> usize {
        let items = self.misc_items as usize;
        self.misc_items = 0;
        if items == 0 || self.misc_len == 0 {
            resp[0] = STLINK_FALSE;
            resp[1] = 0;
            return 2;
        }

        let mut buf_copy = [0u8; 1024];
        let copy_len = self.misc_len.min(buf_copy.len());
        buf_copy[..copy_len].copy_from_slice(&self.misc_buf[..copy_len]);

        let op_padding = (items + 3) & !3;
        let ops = &buf_copy[..items];
        let val_bytes = &buf_copy[op_padding..op_padding + 4 * items];

        let saved_ap = self.current_ap;
        let mut st = STLINK_OK;
        let mut addr = 0u32;

        for i in 0..items {
            let v = get32(&val_bytes[4 * i..4 * (i + 1)]);
            let mut out = 0u32;
            if st == STLINK_OK {
                match ops[i] {
                    MISC_ADDRESS => addr = v,
                    MISC_WRITE => st = self.mem_write32(addr, v),
                    MISC_READ => st = self.mem_read32(v, &mut out),
                    MISC_APNUM => self.current_ap = v as u8,
                    _ => st = STLINK_FALSE,
                }
                self.last_status = st;
            }
            put32(&mut resp[4 * i..4 * (i + 1)], out);
            put32(&mut resp[4 * items + 4 * i..4 * items + 4 * (i + 1)], st as u32);
        }
        self.current_ap = saved_ap;
        8 * items
    }

    /// Execute standard ST-Link command. Writes response to `resp` and returns response length.
    pub fn execute(&mut self, cmd: &[u8], resp: &mut [u8]) -> usize {
        if cmd.is_empty() {
            return 0;
        }

        match cmd[0] {
            STLINK_GET_VERSION => {
                resp[0] = (self.reported_version >> 8) as u8;
                resp[1] = self.reported_version as u8;
                resp[2] = (STLINK_USB_VID & 0xFF) as u8;
                resp[3] = (STLINK_USB_VID >> 8) as u8;
                resp[4] = (STLINK_USB_PID & 0xFF) as u8;
                resp[5] = (STLINK_USB_PID >> 8) as u8;
                6
            }

            STLINK_GET_CURRENT_MODE => {
                resp[0] = self.mode;
                resp[1] = 0;
                2
            }

            STLINK_GET_TARGET_VOLTAGE => {
                let raw_mv = match self.voltage_provider {
                    Some(provider) => provider(),
                    None => 3300,
                };
                let v_target_mv = if raw_mv < 800 { 0 } else { raw_mv };
                put32(&mut resp[0..4], 2400);
                put32(&mut resp[4..8], v_target_mv);
                8
            }

            STLINK_DFU_COMMAND => {
                // DFU EXIT: switch from DFU to MASS
                self.mode = STLINK_MODE_MASS;
                0
            }

            STLINK_BOOTLOADER_COMMAND => {
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            STLINK_DEBUG_COMMAND => self.debug_command(cmd, resp),

            _ => 0,
        }
    }

    fn debug_command(&mut self, cmd: &[u8], resp: &mut [u8]) -> usize {
        if cmd.len() < 2 {
            resp[0] = STLINK_FALSE;
            resp[1] = 0;
            return 2;
        }

        // CmsisDapHandler calls ensure_initialized() at every transfer entry
        // point; ST-Link mode previously never did, leaving the PIO SM or pin
        // directions in a stale state after a personality switch or line init.
        let _ = self.engine.transport.ensure_initialized();

        // Core debug (halt, registers, reset, debug-register access) always
        // runs on AP 0 with the default CSW, whatever a previous memory
        // command left selected. The DAP-level commands manage their own AP
        // state and are exempt. Mirrors hybridlink's debug_command prologue.
        match cmd[1] {
            DBG_APIV2_RW_MISC_IN
            | DBG_APIV2_RW_MISC_OUT
            | DBG_APIV2_INIT_AP
            | DBG_APIV2_CLOSE_AP_DBG
            | DBG_APIV2_READ_DAP_REG
            | DBG_APIV2_WRITE_DAP_REG => {}
            _ => {
                // `mem_access` derives CSW from the address range, so only the
                // cached AP selector needs resetting here.
                self.current_ap = 0;
            }
        }

        match cmd[1] {
            DBG_APIV2_RW_MISC_IN => self.run_misc(resp),

            DBG_APIV1_ENTER | DBG_APIV2_ENTER => {
                if cmd.len() >= 3 && cmd[2] != DBG_ENTER_SWD {
                    resp[0] = STLINK_NO_DEVICE;
                    resp[1] = 0;
                    return 2;
                }
                // Bring up the SWD line before replying. jtag_to_swd() emits
                // the JTAG-to-SWD switch followed by a line reset, matching
                // hybridlink's swd_connect() order. An extra reset_sequence()
                // first is redundant.
                let _ = self.engine.transport.ensure_initialized();
                self.engine.transport.jtag_to_swd();
                // Power the DP here where the line state is known-good. Bounded
                // (~2 ms worst case), unlike read_dpidr_retry.
                let _ = self.engine.power_up_debug();
                // Mark powered regardless: a subsequent host command (IDCODE
                // read or DEBUGREG access) will retry as needed. Re-running
                // power_up_debug on the first AP access right after line reset
                // is what returns AP_FAULT.
                self.dp_powered = true;
                self.mode = STLINK_MODE_DEBUG;
                self.idcode = 0;
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_EXIT => {
                self.mode = STLINK_MODE_MASS;
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_READCOREID => {
                put32(&mut resp[0..4], self.idcode);
                4
            }

            DBG_APIV2_READ_IDCODES => {
                // Deferred target detection. hybridlink does the same: the
                // host asks for IDCODEs once ENTER_SWD has already returned,
                // so the DP probe can take as long as it needs without
                // holding up the handshake.
                if self.idcode == 0 {
                    let _ = self.engine.power_up_debug();
                    let _ = self.engine.select_ap(0, 0);
                    let _ = self.engine.write_ap(AP_CSW, ADIV5_AP_CSW_DEFAULT);
                    let mut id = 0u32;
                    if self.engine.read_dp(DP_DPIDR, &mut id).is_ok()
                        && id != 0
                        && id != 0xFFFF_FFFF
                    {
                        self.idcode = id;
                    }
                }
                resp[..12].fill(0);
                let st = if self.idcode != 0 {
                    STLINK_OK
                } else {
                    STLINK_SWD_DP_ERROR
                };
                resp[0] = st;
                put32(&mut resp[4..8], self.idcode);
                12
            }

            DBG_GETSTATUS => {
                let mut v = 0u32;
                let st = self.mem_read32(DHCSR, &mut v);
                resp[0] = if st != STLINK_OK {
                    st
                } else if (v & S_HALT) != 0 {
                    STLINK_CORE_HALTED
                } else {
                    STLINK_CORE_RUNNING
                };
                resp[1] = 0;
                2
            }

            DBG_FORCEDEBUG => {
                let st = self.core_dhcsr(C_HALT | C_DEBUGEN);
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_RUNCORE => {
                let st = self.core_dhcsr(C_DEBUGEN);
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_STEPCORE => {
                let st = self.core_dhcsr(C_STEP | C_DEBUGEN);
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_APIV1_RESETSYS | DBG_APIV2_RESETSYS => {
                let _ = self.mem_write32(AIRCR, 0x05FA0004);
                delay_cycles(300_000); // ~2 ms at 150 MHz
                let mut dummy = 0u32;
                let _ = self.mem_read32(DHCSR, &mut dummy);
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV2_DRIVE_NRST => {
                if cmd.len() >= 3 {
                    if cmd[2] == 2 {
                        let _ = self.engine.transport.assert_reset(true);
                        delay_cycles(1_500_000); // ~10 ms
                        let _ = self.engine.transport.assert_reset(false);
                    } else {
                        let _ = self.engine.transport.assert_reset(cmd[2] == 0);
                    }
                }
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV1_READREG => {
                let mut v = 0u32;
                let reg = if cmd.len() >= 3 { cmd[2] } else { 0 };
                let _ = self.core_read_reg(reg, &mut v);
                put32(&mut resp[0..4], v);
                4
            }

            DBG_APIV2_READREG => {
                let mut v = 0u32;
                let reg = if cmd.len() >= 3 { cmd[2] } else { 0 };
                let st = self.core_read_reg(reg, &mut v);
                resp[..8].fill(0);
                resp[0] = st;
                put32(&mut resp[4..8], v);
                8
            }

            DBG_APIV1_WRITEREG | DBG_APIV2_WRITEREG => {
                let reg = if cmd.len() >= 3 { cmd[2] } else { 0 };
                let val = if cmd.len() >= 7 { get32(&cmd[3..7]) } else { 0 };
                let st = self.core_write_reg(reg, val);
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_APIV1_READALLREGS | DBG_APIV2_READALLREGS => {
                static REGS: [u8; 20] = [
                    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 20,
                ];
                let off = if cmd[1] == DBG_APIV2_READALLREGS { 4 } else { 0 };
                let total = off + 21 * 4;
                resp[..total].fill(0);
                let mut st = STLINK_OK;
                for (i, &reg) in REGS.iter().enumerate() {
                    let mut val = 0u32;
                    let s = self.core_read_reg(reg, &mut val);
                    if s != STLINK_OK {
                        st = s;
                        break;
                    }
                    put32(&mut resp[off + 4 * i..off + 4 * (i + 1)], val);
                }
                if off > 0 {
                    resp[0] = st;
                }
                total
            }

            DBG_APIV2_READALLREGS_FP => {
                resp[..4 + 33 * 4].fill(0);
                let mut mvfr0 = 0u32;
                let st = self.mem_read32(MVFR0, &mut mvfr0);
                if st == STLINK_OK && mvfr0 != 0 {
                    for i in 0..32 {
                        let mut s = 0u32;
                        let _ = self.core_read_reg(0x40 + i as u8, &mut s);
                        put32(&mut resp[4 + 4 * i..4 + 4 * (i + 1)], s);
                    }
                    let mut fpscr = 0u32;
                    let _ = self.core_read_reg(0x21, &mut fpscr);
                    put32(&mut resp[4 + 32 * 4..4 + 33 * 4], fpscr);
                }
                resp[0] = st;
                4 + 33 * 4
            }

            DBG_APIV1_WRITEDEBUGREG | DBG_APIV2_WRITEDEBUGREG => {
                let addr = if cmd.len() >= 6 { get32(&cmd[2..6]) } else { 0 };
                let val = if cmd.len() >= 10 { get32(&cmd[6..10]) } else { 0 };
                let st = self.mem_write32(addr, val);
                self.last_status = st;
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_APIV2_READDEBUGREG => {
                let addr = if cmd.len() >= 6 { get32(&cmd[2..6]) } else { 0 };
                let mut v = 0u32;
                let st = self.mem_read32(addr, &mut v);
                self.last_status = st;
                resp[..8].fill(0);
                resp[0] = st;
                put32(&mut resp[4..8], v);
                8
            }

            DBG_APIV2_GETLASTRWSTATUS => {
                resp[0] = self.last_status;
                resp[1] = 0;
                2
            }

            DBG_APIV2_GETLASTRWSTATUS2 => {
                resp[..12].fill(0);
                resp[0] = self.last_status;
                self.last_status = STLINK_OK;
                12
            }

            DBG_APIV2_SWD_SET_FREQ => {
                if cmd.len() >= 4 {
                    let div = get16(&cmd[2..4]);
                    let freq_khz = crate::clock::swd_divisor_to_khz(div);
                    let _ = self.engine.transport.set_clock(freq_khz);
                }
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV2_JTAG_SET_FREQ => {
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV2_READ_DAP_REG => {
                let port = if cmd.len() >= 4 { get16(&cmd[2..4]) } else { 0 };
                let reg = if cmd.len() >= 6 { get16(&cmd[4..6]) } else { 0 };
                let mut val = 0u32;
                let st = if port == 0xFFFF {
                    if self.engine.read_dp(reg as u8, &mut val).is_ok() {
                        STLINK_OK
                    } else {
                        STLINK_SWD_DP_FAULT
                    }
                } else {
                    if self.engine.read_ap_port(port as u8, reg as u8, &mut val).is_ok() {
                        STLINK_OK
                    } else {
                        STLINK_SWD_AP_FAULT
                    }
                };
                resp[..8].fill(0);
                resp[0] = st;
                put32(&mut resp[4..8], val);
                8
            }

            DBG_APIV2_WRITE_DAP_REG => {
                let port = if cmd.len() >= 4 { get16(&cmd[2..4]) } else { 0 };
                let reg = if cmd.len() >= 6 { get16(&cmd[4..6]) } else { 0 };
                let val = if cmd.len() >= 10 { get32(&cmd[6..10]) } else { 0 };
                let st = if port == 0xFFFF {
                    if self.engine.write_dp(reg as u8, val).is_ok() {
                        STLINK_OK
                    } else {
                        STLINK_SWD_DP_FAULT
                    }
                } else {
                    if self.engine.write_ap_port(port as u8, reg as u8, val).is_ok() {
                        STLINK_OK
                    } else {
                        STLINK_SWD_AP_FAULT
                    }
                };
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_APIV2_INIT_AP => {
                let ap = if cmd.len() >= 3 { cmd[2] } else { 0 };
                // hybridlink's mem_ap_init selects the AP, invalidates the
                // cached CSW and reads the AP IDR to validate the selection.
                // Merely caching `current_ap` leaves the engine's SELECT/CSW
                // state stale, so later accesses hit the wrong AP.
                self.current_ap = ap;
                // The DP is powered down after a fresh connect. Any AP access
                // before power-up returns WAIT/FAULT, which the host reports
                // as "READDEBUGREG access port fault".
                let _ = self.engine.power_up_debug();
                let mut idr = 0u32;
                let st = self
                    .engine
                    .read_ap_port(ap, 0xFC, &mut idr)
                    .map(|_| STLINK_OK)
                    .unwrap_or(STLINK_SWD_AP_FAULT);
                resp[0] = st;
                resp[1] = 0;
                2
            }

            DBG_APIV2_CLOSE_AP_DBG => {
                self.current_ap = 0;
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV2_START_TRACE_RX => {
                let trace_max = if cmd.len() >= 4 {
                    get16(&cmd[2..4])
                } else {
                    0
                };
                let baud = if cmd.len() >= 8 {
                    get32(&cmd[4..8])
                } else {
                    SWO_DEFAULT_BAUD
                };
                let ok = self.swo.start(baud, trace_max);
                resp[0] = if ok { STLINK_OK } else { STLINK_FALSE };
                resp[1] = 0;
                2
            }

            DBG_APIV2_STOP_TRACE_RX => {
                self.swo.stop();
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            DBG_APIV2_GET_TRACE_NB => {
                let count = self.swo.stlink_trace_count();
                resp[0] = (count & 0xFF) as u8;
                resp[1] = ((count >> 8) & 0xFF) as u8;
                2
            }

            DBG_APIV2_BOARD_INFO => {
                resp[..128].fill(0);
                resp[0] = STLINK_OK;
                let b = b"UNDEFINED  ";
                resp[24..24 + b.len()].copy_from_slice(b);
                128
            }

            DBG_APIV2_GET_CAPS => {
                resp[..36].fill(0);
                resp[0] = STLINK_OK;
                put32(&mut resp[4..8], 2000);
                put32(&mut resp[8..12], 1);
                put32(&mut resp[12..16], 36000);
                put32(&mut resp[16..20], 18);
                put32(&mut resp[20..24], 0xFFFF);
                36
            }

            DBG_APIV2_GET_TRACE_CAPS => {
                resp[..48].fill(0);
                resp[0] = STLINK_OK;
                resp[2] = 0x0A;
                resp[5] = 0x08;
                48
            }

            DBG_APIV1_SETFP
            | DBG_APIV1_CLEARFP
            | DBG_APIV1_SETWATCHPOINT
            | DBG_APIV2_SETFP
            | DBG_APIV2_CLEARFP
            | DBG_APIV2_SETWATCHPOINT => {
                resp[0] = STLINK_OK;
                resp[1] = 0;
                2
            }

            _ => {
                resp[0] = STLINK_FALSE;
                resp[1] = 0;
                2
            }
        }
    }
}

/// Diagnostics for the ST-Link MEM-AP path. `step` records the last stage
/// reached, `fail` the failing stage, `ok` the last successful address.
pub mod stlink_diag {
    use core::sync::atomic::{AtomicU32, Ordering};
    pub static STEP: AtomicU32 = AtomicU32::new(0);
    pub static FAIL: AtomicU32 = AtomicU32::new(0);
    pub static LAST_OK: AtomicU32 = AtomicU32::new(0);
    pub static LAST_ADDR: AtomicU32 = AtomicU32::new(0);
    pub static CTRLSTAT: AtomicU32 = AtomicU32::new(0);
    pub static WRITES: AtomicU32 = AtomicU32::new(0);
    pub static LAST_ACK: AtomicU32 = AtomicU32::new(0);
    pub fn step(s: u32) { STEP.store(s, Ordering::Relaxed); }
    pub fn fail(s: u32) { FAIL.store(s, Ordering::Relaxed); }
    pub fn ok(st: u8, addr: u32) {
        if st == STLINK_OK { LAST_OK.store(1, Ordering::Relaxed); }
        LAST_ADDR.store(addr, Ordering::Relaxed);
    }
    pub fn set_ctrlstat(v: u32) { CTRLSTAT.store(v, Ordering::Relaxed); }
    pub fn set_writes(v: u32) { WRITES.store(v, Ordering::Relaxed); }
    pub fn set_last_ack(a: u8, reg: u8) { LAST_ACK.store(((reg as u32)<<8)|a as u32, Ordering::Relaxed); }
    pub static LAST_REQ: AtomicU32 = AtomicU32::new(0);
    pub static LAST_VAL: AtomicU32 = AtomicU32::new(0);
    pub fn set_req(req: u8, val: u32) { LAST_REQ.store(req as u32, Ordering::Relaxed); LAST_VAL.store(val, Ordering::Relaxed); }
    use super::STLINK_OK;
}
