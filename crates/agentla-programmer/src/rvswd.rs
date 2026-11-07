//! WCH QingKe 2-wire RVSWD protocol definitions and abstractions.

use crate::traits::RvswdIo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RvswdError {
    Timeout,
    ParityError,
    NotInit,
}

pub fn build_header(reg: u8, is_write: bool) -> u32 {
    let mut packet = (reg as u32) & 0x7F;
    let mut parity = false;
    for i in 0..7 {
        if ((packet >> (6 - i)) & 1) != 0 {
            parity = !parity;
        }
    }
    packet = (packet << 1) | (if is_write { 1 } else { 0 });
    if is_write {
        parity = !parity;
    }
    (packet << 1) | (if parity { 1 } else { 0 })
}

/// Generic high-level WCH 2-wire RVSWD session operating over any `RvswdIo` transport.
pub struct RvswdEngine<T: RvswdIo> {
    pub transport: T,
    pub initialized: bool,
}

impl<T: RvswdIo> RvswdEngine<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            initialized: false,
        }
    }

    pub fn attach(&mut self) -> Result<u32, RvswdError> {
        self.transport.reset().map_err(|_| RvswdError::Timeout)?;
        self.initialized = true;
        self.read(0x11)
    }

    pub fn write(&mut self, reg: u8, value: u32) -> Result<(), RvswdError> {
        self.transport.write(reg, value).map_err(|_| RvswdError::Timeout)
    }

    pub fn read(&mut self, reg: u8) -> Result<u32, RvswdError> {
        self.transport.read(reg).map_err(|_| RvswdError::Timeout)
    }
}
