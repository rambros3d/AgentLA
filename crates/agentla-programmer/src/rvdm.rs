//! RISC-V 0.13 Debug Module (RVDM) commands, abstract registers, and memory streaming.
//!
//! Provides halt, resume, register access, and memory read/write streaming through the
//! RISC-V Debug Module (DM) program buffer over any `RvswdIo` transport.

use crate::delay::delay_cycles;
use crate::traits::RvswdIo;

pub const DM_DATA0: u8 = 0x04;
pub const DM_DMCONTROL: u8 = 0x10;
pub const DM_DMSTATUS: u8 = 0x11;
pub const DM_HARTINFO: u8 = 0x12;
pub const DM_ABSTRACTCS: u8 = 0x16;
pub const DM_COMMAND: u8 = 0x17;
pub const DM_PROGBUF0: u8 = 0x20;
pub const DM_WCH_CHIPID: u8 = 0x7F;

pub const DMCONTROL_HALTREQ: u32 = 1 << 31;
pub const DMCONTROL_RESUMEREQ: u32 = 1 << 30;
pub const DMCONTROL_ACKHAVERESET: u32 = 1 << 28;
pub const DMCONTROL_NDMRESET: u32 = 1 << 1;
pub const DMCONTROL_DMACTIVE: u32 = 1 << 0;

pub const DMSTATUS_ALLRESUMEACK: u32 = 1 << 17;
pub const DMSTATUS_ALLHAVERESET: u32 = 1 << 19;
pub const DMSTATUS_ALLRUNNING: u32 = 1 << 11;
pub const DMSTATUS_ALLHALTED: u32 = 1 << 9;

pub const ABSTRACTCS_BUSY: u32 = 1 << 12;
pub const ABSTRACTCS_CMDERR: u32 = 7 << 8;

pub const CMD_REG: u32 = (2 << 20) | (1 << 17);
pub const CMD_WRITE: u32 = 1 << 16;
pub const CMD_POSTEXEC: u32 = 1 << 18;
pub const REG_S0: u32 = 0x1008;
pub const REG_S1: u32 = 0x1009;

pub const INSN_LW_S1_S0: u32 = 0x00042483; // lw s1, 0(s0)
pub const INSN_SW_S1_S0: u32 = 0x00942023; // sw s1, 0(s0)
pub const INSN_SH_S1_S0: u32 = 0x00941023; // sh s1, 0(s0)
pub const INSN_EBREAK: u32 = 0x00100073;   // ebreak

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RvdmError {
    TransportError,
    BusyTimeout,
    CommandError(u8),
    NotHalted,
}

pub struct Rvdm<T: RvswdIo> {
    pub io: T,
    pub saved_s0: u32,
    pub saved_s1: u32,
    pub progbuf_op: u32,
}

impl<T: RvswdIo> Rvdm<T> {
    pub const fn new(io: T) -> Self {
        Self {
            io,
            saved_s0: 0,
            saved_s1: 0,
            progbuf_op: 0,
        }
    }

    #[inline]
    pub fn dmi_read(&mut self, addr: u8) -> Result<u32, RvdmError> {
        self.io.read(addr).map_err(|_| RvdmError::TransportError)
    }

    #[inline]
    pub fn dmi_write(&mut self, addr: u8, val: u32) -> Result<(), RvdmError> {
        self.io.write(addr, val).map_err(|_| RvdmError::TransportError)
    }

    pub fn attach(&mut self) -> Result<u32, RvdmError> {
        self.io.reset().map_err(|_| RvdmError::TransportError)?;
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE)?;
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE)?;
        self.progbuf_op = 0;
        self.dmi_read(DM_WCH_CHIPID)
    }

    pub fn wait_status(&mut self, mask: u32, timeout_cycles: u32) -> Result<(), RvdmError> {
        let retries = timeout_cycles.max(100);
        for _ in 0..retries {
            let s = self.dmi_read(DM_DMSTATUS)?;
            if (s & mask) != 0 {
                return Ok(());
            }
            delay_cycles(150);
        }
        Err(RvdmError::BusyTimeout)
    }

    pub fn is_halted(&mut self) -> Result<bool, RvdmError> {
        let s = self.dmi_read(DM_DMSTATUS)?;
        Ok((s & DMSTATUS_ALLHALTED) != 0)
    }

    pub fn halt(&mut self) -> Result<(), RvdmError> {
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE | DMCONTROL_HALTREQ)?;
        let res = self.wait_status(DMSTATUS_ALLHALTED, 2000);
        let _ = self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE);
        res
    }

    pub fn resume(&mut self) -> Result<(), RvdmError> {
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE | DMCONTROL_RESUMEREQ)?;
        let res = self.wait_status(DMSTATUS_ALLRESUMEACK, 2000);
        let _ = self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE);
        res
    }

    pub fn reset(&mut self, halt: bool) -> Result<(), RvdmError> {
        let h = if halt { DMCONTROL_HALTREQ } else { 0 };
        // Free abstract command stuck on a bus access
        let _ = self.dmi_write(DM_DMCONTROL, 0);
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE)?;
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE | DMCONTROL_NDMRESET | h)?;
        delay_cycles(15000);
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE | h)?;
        if halt {
            self.wait_status(DMSTATUS_ALLHALTED, 2000)?;
        }
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE | DMCONTROL_ACKHAVERESET)?;
        self.dmi_write(DM_DMCONTROL, DMCONTROL_DMACTIVE)?;
        self.progbuf_op = 0;
        Ok(())
    }

    pub fn command(&mut self, cmd: u32) -> Result<(), RvdmError> {
        self.dmi_write(DM_COMMAND, cmd)?;
        let mut cs = 0u32;
        for _ in 0..100 {
            cs = self.dmi_read(DM_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                break;
            }
        }
        if (cs & (ABSTRACTCS_BUSY | ABSTRACTCS_CMDERR)) != 0 {
            let _ = self.dmi_write(DM_ABSTRACTCS, ABSTRACTCS_CMDERR);
            return Err(RvdmError::CommandError(((cs >> 8) & 7) as u8));
        }
        Ok(())
    }

    pub fn reg_read(&mut self, reg: u32) -> Result<u32, RvdmError> {
        self.command(CMD_REG | reg)?;
        self.dmi_read(DM_DATA0)
    }

    pub fn reg_write(&mut self, reg: u32, val: u32, exec: bool) -> Result<(), RvdmError> {
        self.dmi_write(DM_DATA0, val)?;
        self.command(CMD_REG | CMD_WRITE | (if exec { CMD_POSTEXEC } else { 0 }) | reg)
    }

    pub fn progbuf(&mut self, insn: u32) -> Result<(), RvdmError> {
        if self.progbuf_op == insn {
            return Ok(());
        }
        self.dmi_write(DM_PROGBUF0, insn)?;
        self.dmi_write(DM_PROGBUF0 + 1, INSN_EBREAK)?;
        self.progbuf_op = insn;
        Ok(())
    }

    pub fn begin(&mut self) -> Result<bool, RvdmError> {
        let was_running = !self.is_halted()?;
        if was_running {
            self.halt()?;
        }
        self.progbuf_op = 0;
        let _ = self.dmi_write(DM_ABSTRACTCS, ABSTRACTCS_CMDERR);
        self.saved_s0 = self.reg_read(REG_S0)?;
        self.saved_s1 = self.reg_read(REG_S1)?;
        Ok(was_running)
    }

    pub fn end(&mut self, was_running: bool) -> Result<(), RvdmError> {
        let _ = self.reg_write(REG_S0, self.saved_s0, false);
        let _ = self.reg_write(REG_S1, self.saved_s1, false);
        if was_running {
            self.resume()?;
        }
        Ok(())
    }

    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, RvdmError> {
        self.progbuf(INSN_LW_S1_S0)?;
        self.reg_write(REG_S0, addr, true)?;
        self.reg_read(REG_S1)
    }

    pub fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), RvdmError> {
        self.progbuf(INSN_SW_S1_S0)?;
        self.reg_write(REG_S1, val, false)?;
        self.reg_write(REG_S0, addr, true)
    }

    pub fn mem_write16(&mut self, addr: u32, val: u16) -> Result<(), RvdmError> {
        self.progbuf(INSN_SH_S1_S0)?;
        self.reg_write(REG_S1, val as u32, false)?;
        self.reg_write(REG_S0, addr, true)
    }

    /// Stream words into target memory using the program buffer loop.
    /// After each word, the target spins until `(status_reg & mask) == 0`.
    pub fn mem_stream32(
        &mut self,
        addr: u32,
        words: &[u32],
        busy_reg: u32,
        mask: u32,
    ) -> Result<(), RvdmError> {
        if words.is_empty() {
            return Ok(());
        }
        let hi = (busy_reg.wrapping_add(0x800)) >> 12;
        let lo = busy_reg.wrapping_sub(hi << 12);
        let prog: [u32; 7] = [
            INSN_SW_S1_S0,
            0x00440413, // addi s0, s0, 4
            (hi << 12) | (9 << 7) | 0x37, // lui s1, hi
            ((lo & 0xFFF) << 20) | (9 << 15) | (2 << 12) | (9 << 7) | 0x03, // lw s1, lo(s1)
            ((mask & 0x7FF) << 20) | (9 << 15) | (7 << 12) | (9 << 7) | 0x13, // andi s1, s1, mask
            0xFE049AE3, // bnez s1, -12
            INSN_EBREAK,
        ];
        self.progbuf_op = 0;
        for (i, &insn) in prog.iter().enumerate() {
            self.dmi_write(DM_PROGBUF0 + (i as u8), insn)?;
        }
        self.reg_write(REG_S0, addr, false)?;
        for &word in words {
            self.dmi_write(DM_DATA0, word)?;
            self.command(CMD_REG | CMD_WRITE | CMD_POSTEXEC | REG_S1)?;
        }
        Ok(())
    }

    /// Stream bytes (interpreted as little-endian 32-bit words) into target memory.
    pub fn mem_stream32_bytes(
        &mut self,
        addr: u32,
        bytes: &[u8],
        busy_reg: u32,
        mask: u32,
    ) -> Result<(), RvdmError> {
        let n_words = bytes.len() / 4;
        if n_words == 0 {
            return Ok(());
        }
        let hi = (busy_reg.wrapping_add(0x800)) >> 12;
        let lo = busy_reg.wrapping_sub(hi << 12);
        let prog: [u32; 7] = [
            INSN_SW_S1_S0,
            0x00440413, // addi s0, s0, 4
            (hi << 12) | (9 << 7) | 0x37, // lui s1, hi
            ((lo & 0xFFF) << 20) | (9 << 15) | (2 << 12) | (9 << 7) | 0x03, // lw s1, lo(s1)
            ((mask & 0x7FF) << 20) | (9 << 15) | (7 << 12) | (9 << 7) | 0x13, // andi s1, s1, mask
            0xFE049AE3, // bnez s1, -12
            INSN_EBREAK,
        ];
        self.progbuf_op = 0;
        for (i, &insn) in prog.iter().enumerate() {
            self.dmi_write(DM_PROGBUF0 + (i as u8), insn)?;
        }
        self.reg_write(REG_S0, addr, false)?;
        for i in 0..n_words {
            let chunk = &bytes[i * 4..(i + 1) * 4];
            let word = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            self.dmi_write(DM_DATA0, word)?;
            self.command(CMD_REG | CMD_WRITE | CMD_POSTEXEC | REG_S1)?;
        }
        Ok(())
    }
}
