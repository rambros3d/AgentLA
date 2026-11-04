use crate::programmer::rvswd::RvswdTransport;
use crate::programmer::swio::SwioTransport;

pub const DMI_DMCONTROL: u8 = 0x10;
pub const DMI_DMSTATUS: u8 = 0x11;
pub const DMI_HARTINFO: u8 = 0x12;
pub const DMI_ABSTRACTCS: u8 = 0x16;
pub const DMI_COMMAND: u8 = 0x17;
pub const DMI_ABSTRACTAUTO: u8 = 0x18;
pub const DMI_PROGBUF0: u8 = 0x20;
pub const DMI_DATA0: u8 = 0x04;
pub const DMI_DATA1: u8 = 0x05;

pub const DMCONTROL_HALTREQ: u32 = 1 << 31;
pub const DMCONTROL_RESUMEREQ: u32 = 1 << 30;
pub const DMCONTROL_HARTSEL: u32 = 0x3FF << 16;
pub const DMCONTROL_DMACTIVE: u32 = 1 << 0;

pub const DMSTATUS_ALLHALTED: u32 = 1 << 9;
pub const DMSTATUS_ALLRUNNING: u32 = 1 << 10;
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
}

pub enum QingKeWire<'a> {
    OneWire(&'a mut SwioTransport),
    TwoWire(&'a mut RvswdTransport),
}

pub struct QingKe<'a> {
    pub wire: QingKeWire<'a>,
}

impl<'a> QingKe<'a> {
    pub fn new_1wire(swio: &'a mut SwioTransport) -> Self {
        Self {
            wire: QingKeWire::OneWire(swio),
        }
    }

    pub fn new_2wire(rvswd: &'a mut RvswdTransport) -> Self {
        Self {
            wire: QingKeWire::TwoWire(rvswd),
        }
    }

    pub fn dmi_read(&mut self, addr: u8) -> Result<u32, QingKeError> {
        match &mut self.wire {
            QingKeWire::OneWire(swio) => swio
                .get(addr as u32)
                .map_err(|_| QingKeError::WireError),
            QingKeWire::TwoWire(rvswd) => rvswd
                .read(addr)
                .map_err(|_| QingKeError::WireError),
        }
    }

    pub fn dmi_write(&mut self, addr: u8, data: u32) -> Result<(), QingKeError> {
        match &mut self.wire {
            QingKeWire::OneWire(swio) => {
                swio.put(addr as u32, data);
                Ok(())
            }
            QingKeWire::TwoWire(rvswd) => rvswd
                .write(addr, data)
                .map_err(|_| QingKeError::WireError),
        }
    }

    pub fn halt(&mut self) -> Result<(), QingKeError> {
        // Set haltreq | dmactive
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_HALTREQ | DMCONTROL_DMACTIVE)?;
        for _ in 0..1000 {
            let status = self.dmi_read(DMI_DMSTATUS)?;
            if (status & DMSTATUS_ALLHALTED) != 0 {
                // Clear haltreq
                self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE)?;
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn resume(&mut self) -> Result<(), QingKeError> {
        // Set resumereq | dmactive
        self.dmi_write(DMI_DMCONTROL, DMCONTROL_RESUMEREQ | DMCONTROL_DMACTIVE)?;
        for _ in 0..1000 {
            let status = self.dmi_read(DMI_DMSTATUS)?;
            if (status & DMSTATUS_ALLRUNNING) != 0 {
                self.dmi_write(DMI_DMCONTROL, DMCONTROL_DMACTIVE)?;
                return Ok(());
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn is_halted(&mut self) -> Result<bool, QingKeError> {
        let status = self.dmi_read(DMI_DMSTATUS)?;
        Ok((status & DMSTATUS_ALLHALTED) != 0)
    }

    pub fn read_reg(&mut self, regno: u16) -> Result<u32, QingKeError> {
        let cmd = CMD_TYPE_ACCESS_REG | CMD_AARSIZE_32 | CMD_TRANSFER | (regno as u32 & 0xFFFF);
        self.dmi_write(DMI_COMMAND, cmd)?;

        for _ in 0..1000 {
            let cs = self.dmi_read(DMI_ABSTRACTCS)?;
            if (cs & ABSTRACTCS_BUSY) == 0 {
                if (cs & ABSTRACTCS_CMDERR_MASK) != 0 {
                    self.dmi_write(DMI_ABSTRACTCS, ABSTRACTCS_CMDERR_MASK)?;
                    return Err(QingKeError::CommandError);
                }
                return self.dmi_read(DMI_DATA0);
            }
            cortex_m::asm::delay(1500);
        }
        Err(QingKeError::Timeout)
    }

    pub fn write_reg(&mut self, regno: u16, val: u32) -> Result<(), QingKeError> {
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

    pub fn exec_progbuf(&mut self, prog: &[u32]) -> Result<(), QingKeError> {
        let count = prog.len().min(8);
        for i in 0..count {
            self.dmi_write(DMI_PROGBUF0 + i as u8, prog[i])?;
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
}
