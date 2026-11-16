//! WCH QingKe 2-wire RVSWD protocol definitions and abstractions.
//!
//! RVSWD is a half-duplex synchronous wire protocol used by WCH CH32V microcontrollers
//! (CH32V1xx/2xx/3xx) on PA13 (DIO) and PA14 (CLK).
//!
//! Wire protocol rules:
//! - Clock idles high; host changes DIO while CLK is low; target samples on rising edge; MSB first.
//! - START: DIO falls while CLK is high.
//! - Header: addr[7] rw[1] (1 = write, 0 = read), double even parity[2], 4 padding zeros.
//! - Write: host data[32], double even parity[2]; target status[4] (0111 = OK).
//! - Read:  target data[32], target even parity[1], 1, target status[4] (0111 = OK).
//! - STOP:  DIO rises while CLK is high.

use crate::traits::RvswdIo;

/// Target status acknowledgment nibble signifying successful transfer (0b0111).
pub const STATUS_OK: u8 = 0x07;

/// DMI status response codes.
pub const DMI_OK: u8 = 0;
pub const DMI_FAILED: u8 = 2;
pub const DMI_BUSY: u8 = 3;

/// Count of padding zeros following the header parity bits.
pub const PADDING_ZEROS_COUNT: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RvswdError {
    Timeout,
    ParityError,
    StatusError(u8),
    NotInit,
    DmiFailed,
}

/// Calculate even parity bit for a value.
/// Returns 1 if the count of set bits in `val` is odd (so adding 1 makes the total even),
/// or 0 if the count of set bits is already even.
#[inline]
pub const fn even_parity_bit(val: u32) -> u8 {
    (val.count_ones() & 1) as u8
}

/// Calculate doubled even parity (2 bits: 0b11 or 0b00).
#[inline]
pub const fn double_even_parity(val: u32) -> u8 {
    if (val.count_ones() & 1) != 0 {
        0b11
    } else {
        0b00
    }
}

/// Build the 8-bit header byte: `((addr & 0x7F) << 1) | (is_write as u8)`.
#[inline]
pub const fn build_header_byte(addr: u8, is_write: bool) -> u8 {
    ((addr & 0x7F) << 1) | (if is_write { 1 } else { 0 })
}

/// Calculate the 2-bit doubled even parity for a header byte.
#[inline]
pub const fn header_parity(header_byte: u8) -> u8 {
    double_even_parity(header_byte as u32)
}

/// Legacy/convenience helper building header bits.
#[inline]
pub const fn build_header(reg: u8, is_write: bool) -> u32 {
    let h = build_header_byte(reg, is_write) as u32;
    let par = double_even_parity(h) as u32;
    (h << 2) | par
}

/// Verify target status nibble (must equal 0x07 / 0b0111).
#[inline]
pub fn verify_status(status_nibble: u8) -> Result<(), RvswdError> {
    if (status_nibble & 0x0F) == STATUS_OK {
        Ok(())
    } else {
        Err(RvswdError::StatusError(status_nibble & 0x0F))
    }
}

/// Verify read tail consisting of:
/// - bit 5: target even parity bit
/// - bit 4: constant 1
/// - bits [3:0]: status nibble (must equal STATUS_OK)
#[inline]
pub fn verify_read_tail(data: u32, tail: u8) -> Result<(), RvswdError> {
    let status = tail & 0x0F;
    if status != STATUS_OK {
        return Err(RvswdError::StatusError(status));
    }
    let target_parity = (tail >> 5) & 1;
    let expected_parity = even_parity_bit(data);
    if target_parity != expected_parity {
        return Err(RvswdError::ParityError);
    }
    Ok(())
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
