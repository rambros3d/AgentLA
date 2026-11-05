use crate::programmer::rvswd::RvswdTransport;
use crate::programmer::swio::SwioTransport;

pub const DMI_DMCONTROL: u8 = 0x10;
pub const DMI_DMSTATUS: u8 = 0x11;
pub const DMI_HARTINFO: u8 = 0x12;
pub const DMI_ABSTRACTCS: u8 = 0x16;
pub const DMI_COMMAND: u8 = 0x17;
pub const DMI_ABSTRACTAUTO: u8 = 0x18;
pub const DMI_PROGBUF0: u8 = 0x20;
pub const DMI_PROGBUF1: u8 = 0x21;
pub const DMI_DATA0: u8 = 0x04;
pub const DMI_DATA1: u8 = 0x05;
pub const DMI_SBCS: u8 = 0x38;
pub const DMI_SBADDRESS0: u8 = 0x39;
pub const DMI_SBDATA0: u8 = 0x3C;

pub const SBCS_SBBUSY: u32 = 1 << 21;
pub const SBCS_SBREADONADDR: u32 = 1 << 20;
pub const SBCS_SBACCESS_32: u32 = 2 << 17;

pub const DMCONTROL_HALTREQ: u32 = 1 << 31;
pub const DMCONTROL_RESUMEREQ: u32 = 1 << 30;
pub const DMCONTROL_HARTSEL: u32 = 0x3FF << 16;
pub const DMCONTROL_DMACTIVE: u32 = 1 << 0;

pub const DMSTATUS_ALLHALTED: u32 = 1 << 9;
pub const DMSTATUS_ALLRUNNING: u32 = (1 << 10) | (1 << 11);
pub const DMSTATUS_VERSION_MASK: u32 = 0x0F;

pub const ABSTRACTCS_BUSY: u32 = 1 << 12;
pub const ABSTRACTCS_CMDERR_MASK: u32 = 0x7 << 8;

pub const CMD_TYPE_ACCESS_REG: u32 = 0 << 24;
pub const CMD_AARSIZE_32: u32 = 2 << 20;
pub const CMD_POSTEXEC: u32 = 1 << 18;
pub const CMD_TRANSFER: u32 = 1 << 17;
pub const CMD_WRITE: u32 = 1 << 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QingKeError {
    WireError,
    Timeout,
    CommandError,
    CommandErrorCode(u32),
    RvswdParity,
    RvswdTimeout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QingKeFamily {
    V003,
    V20x,
}

pub enum QingKeWire<'a> {
    OneWire(&'a mut SwioTransport),
    TwoWire(&'a mut RvswdTransport),
}

pub struct QingKe<'a> {
    pub wire: QingKeWire<'a>,
    pub family: QingKeFamily,
}

impl<'a> QingKe<'a> {
    pub fn new_1wire(swio: &'a mut SwioTransport) -> Self {
        Self {
            wire: QingKeWire::OneWire(swio),
            family: QingKeFamily::V003,
        }
    }

    pub fn new_2wire(rvswd: &'a mut RvswdTransport) -> Self {
        Self {
            wire: QingKeWire::TwoWire(rvswd),
            family: QingKeFamily::V20x,
        }
    }

    pub fn dmi_read(&mut self, addr: u8) -> Result<u32, QingKeError> {
        match &mut self.wire {
            QingKeWire::OneWire(swio) => swio
                .get(addr as u32)
                .map_err(|_| QingKeError::WireError),
            QingKeWire::TwoWire(rvswd) => {
                let mut last_err = QingKeError::RvswdTimeout;
                for _ in 0..5 {
                    match rvswd.read(addr) {
                        Ok(val) => return Ok(val),
                        Err(crate::programmer::rvswd::RvswdError::ParityError) => {
                            last_err = QingKeError::RvswdParity;
                        }
                        Err(crate::programmer::rvswd::RvswdError::Timeout) => {
                            last_err = QingKeError::RvswdTimeout;
                        }
                        _ => last_err = QingKeError::WireError,
                    }
                    cortex_m::asm::delay(7500); // 50 µs
                }
                Err(last_err)
            }
        }
    }

    pub fn dmi_write(&mut self, addr: u8, data: u32) -> Result<(), QingKeError> {
        match &mut self.wire {
            QingKeWire::OneWire(swio) => {
                swio.put(addr as u32, data);
                Ok(())
            }
            QingKeWire::TwoWire(rvswd) => {
                let mut last_err = QingKeError::WireError;
                for _ in 0..5 {
                    match rvswd.write(addr, data) {
                        Ok(()) => return Ok(()),
                        Err(_) => last_err = QingKeError::WireError,
                    }
                    cortex_m::asm::delay(7500); // 50 µs
                }
                Err(last_err)
            }
        }
    }

    pub fn halt(&mut self) -> Result<(), QingKeError> {
        // Double write HALTREQ | DMACTIVE (matches C reference ch32v20x_halt_microprocessor)
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE)?;
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE)?;
        for _ in 0..1000 {
            let status = self.dmi_read(DMI_DMSTATUS)?;
            if (status & (DMSTATUS_ALLHALTED | (1 << 8))) != 0 {
                // Clear haltreq so core stays halted and PROGBUF can execute cleanly
                let _ = self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE);
                let _ = self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK);
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn resume(&mut self) -> Result<(), QingKeError> {
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE)?;
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE)?;
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE)?;
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_RESUMEREQ | DMCONTROL_DMACTIVE)?;
        for _ in 0..1000 {
            let status = self.dmi_read(DMI_DMSTATUS)?;
            if (status & DMSTATUS_ALLRUNNING) != 0 {
                let _ = self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE);
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        let _ = self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE);
        Ok(())
    }

    pub fn is_halted(&mut self) -> Result<bool, QingKeError> {
        let status = self.dmi_read(DMI_DMSTATUS)?;
        if status == 0 || status == 0xFFFFFFFF {
            return Ok(false);
        }
        Ok((status & (DMSTATUS_ALLHALTED | (1 << 8))) != 0)
    }

    pub fn read_reg(&mut self, regno: u16) -> Result<u32, QingKeError> {
        let cs = self.dmi_read(DMI_ABSTRACTCS)?;
        if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
            self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
        }

        let cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_TRANSFER | (regno as u32 & 0xFFFF);
        self.dmi_write(DMI_COMMAND, cmd)?;

        for _ in 0..1000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                let cmderr = (cs & ABSTRACTCS_CMDERR_MASK) >> 8;
                if cmderr != 0 {
                    self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
                    return Err(QingKeError::CommandErrorCode(500 + cmderr));
                }
                return self.dmi_read(DMI_DATA0);
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn write_reg(&mut self, regno: u16, val: u32) -> Result<(), QingKeError> {
        let cs = self.dmi_read(DMI_ABSTRACTCS)?;
        if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
            self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
        }

        self.dmi_write(DMI_DATA0, val)?;
        let cmd = CMD_TYPE_ACCESS_REG
            | CMD_AARSIZE_32
            | CMD_TRANSFER
            | CMD_WRITE
            | (regno as u32 & 0xFFFF);
        self.dmi_write(DMI_COMMAND, cmd)?;

        for _ in 0..1000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                let cmderr = (cs & ABSTRACTCS_CMDERR_MASK) >> 8;
                if cmderr != 0 {
                    self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
                    return Err(QingKeError::CommandErrorCode(600 + cmderr));
                }
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn exec_progbuf(&mut self, prog: &[u32]) -> Result<(), QingKeError> {
        let count = prog.len().min(8);
        for i in 0..count {
            self.dmi_write(DMI_PROGBUF0 + i as u8, prog[i])?;
        }

        let cs = self.dmi_read(DMI_ABSTRACTCS)?;
        if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
            self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
        }

        let cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_POSTEXEC;
        self.dmi_write(DMI_COMMAND, cmd)?;

        for _ in 0..2000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
                    self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
                    return Err(QingKeError::CommandError);
                }
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    /// Read 32-bit word using proven RISC-V PROGBUF method:
    /// PROGBUF0 = 0x0005a503 [lw a0, 0(a1)]
    /// PROGBUF1 = 0x00100073 [ebreak]
    /// write a1 = addr
    /// execute PROGBUF with postexec
    /// read a0
    pub fn mem_read32_halted(&mut self, addr: u32) -> Result<u32, QingKeError> {
        self.dmi_write(DMI_PROGBUF0, 0x0005a503)?;
        self.dmi_write(DMI_PROGBUF1, 0x00100073)?;
        self.write_reg(0x100B, addr)?;
        let cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_POSTEXEC;
        self.dmi_write(DMI_COMMAND, cmd)?;
        for _ in 0..1000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
                    let _ = self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK);
                }
                break;
            }
            cortex_m::asm::delay(1500);
        }
        self.read_reg(0x100A)
    }

    /// Write 32-bit word using proven RISC-V PROGBUF method:
    /// PROGBUF0 = 0x00a5a023 [sw a0, 0(a1)]
    /// PROGBUF1 = 0x00100073 [ebreak]
    /// write a1 = addr
    /// write a0 = val
    /// execute PROGBUF with postexec
    pub fn mem_write32_halted(&mut self, addr: u32, val: u32) -> Result<(), QingKeError> {
        self.dmi_write(DMI_PROGBUF0, 0x00a5a023)?;
        self.dmi_write(DMI_PROGBUF1, 0x00100073)?;
        self.write_reg(0x100B, addr)?;
        self.write_reg(0x100A, val)?;
        let cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_POSTEXEC;
        self.dmi_write(DMI_COMMAND, cmd)?;
        for _ in 0..1000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
                    let _ = self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK);
                }
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    /// Read 32-bit word using proven RISC-V PROGBUF method:
    /// PROGBUF0 = 0x0005a503 (lw a0, 0(a1))
    /// PROGBUF1 = 0x00100073 (ebreak)
    /// a1 = addr
    /// execute PROGBUF with postexec
    /// return a0
    pub fn mem_read32(&mut self, addr: u32) -> Result<u32, QingKeError> {
        let was_halted = self.is_halted().unwrap_or(false);
        if !was_halted {
            self.halt()?;
        }

        let res = self.mem_read32_halted(addr);

        if !was_halted {
            let _ = self.resume();
        }
        res
    }

    /// Write 32-bit word using PROGBUF:
    /// PROGBUF0 = 0x00a5a023 (sw a0, 0(a1))
    /// PROGBUF1 = 0x00100073 (ebreak)
    /// a1 = addr
    /// a0 = val
    /// execute PROGBUF with postexec
    pub fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), QingKeError> {
        let was_halted = self.is_halted().unwrap_or(false);
        if !was_halted {
            self.halt()?;
        }

        let res = self.mem_write32_halted(addr, val);

        if !was_halted {
            let _ = self.resume();
        }
        res
    }

    pub fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), QingKeError> {
        let was_halted = self.is_halted().unwrap_or(false);
        if !was_halted {
            self.halt()?;
        }

        let mut offset = 0usize;
        let mut curr_addr = addr;
        let mut err = None;
        while offset + 4 <= buf.len() {
            match self.mem_read32_halted(curr_addr) {
                Ok(val) => {
                    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
                    offset += 4;
                    curr_addr += 4;
                }
                Err(e) => {
                    err = Some(e);
                    break;
                }
            }
        }
        if err.is_none() && offset < buf.len() {
            match self.mem_read32_halted(curr_addr) {
                Ok(val) => {
                    let bytes = val.to_le_bytes();
                    let rem = buf.len() - offset;
                    buf[offset..offset + rem].copy_from_slice(&bytes[..rem]);
                }
                Err(e) => {
                    err = Some(e);
                }
            }
        }

        if !was_halted {
            let _ = self.resume();
        }

        if let Some(e) = err {
            Err(e)
        } else {
            Ok(())
        }
    }
}
