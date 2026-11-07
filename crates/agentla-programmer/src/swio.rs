//! WCH QingKe 1-wire SWIO protocol definitions and abstractions.

use crate::traits::SwioIo;

pub const WCH_DM_CPBR: u32 = 0x7C;
pub const WCH_DM_CFGR: u32 = 0x7D;
pub const WCH_DM_SHDWCFGR: u32 = 0x7E;
pub const WCH_DM_PART: u32 = 0x7F;

pub const DM_DMCONTROL: u32 = 0x10;
pub const DM_DMSTATUS: u32 = 0x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwioError {
    Timeout,
    NotInit,
}

/// Generic high-level WCH SWIO session operating over any `SwioIo` transport.
pub struct SwioEngine<T: SwioIo> {
    pub transport: T,
    pub initialized: bool,
}

impl<T: SwioIo> SwioEngine<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            initialized: false,
        }
    }

    pub fn attach(&mut self) -> Result<u32, SwioError> {
        self.transport
            .reset_sequence()
            .map_err(|_| SwioError::Timeout)?;
        self.put(WCH_DM_SHDWCFGR, 0x5AA50400);
        self.put(WCH_DM_CFGR, 0x5AA50400);
        self.put(DM_DMCONTROL, 0x00000000);
        self.put(DM_DMCONTROL, 0x00000001);
        self.initialized = true;
        self.get(DM_DMSTATUS)
    }

    pub fn put(&mut self, addr: u32, data: u32) {
        let _ = self.transport.put(addr, data);
    }

    pub fn get(&mut self, addr: u32) -> Result<u32, SwioError> {
        self.transport.get(addr).map_err(|_| SwioError::Timeout)
    }
}
