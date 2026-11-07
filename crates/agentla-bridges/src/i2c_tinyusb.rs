//! Linux i2c-tiny-usb protocol engine.
//!
//! Conforms to Linux kernel `i2c-tiny-usb` protocol (`drivers/i2c/busses/i2c-tiny-usb.c`).
//! Provides request dispatching over `embedded-hal::i2c::I2c`.

use embedded_hal::i2c::I2c;

pub const CMD_ECHO: u8 = 0;
pub const CMD_GET_FUNC: u8 = 1;
pub const CMD_SET_DELAY: u8 = 2;
pub const CMD_GET_STATUS: u8 = 3;
pub const CMD_I2C_IO: u8 = 4;

pub const CMD_I2C_IO_BEGIN: u8 = 1 << 0; // 0x01
pub const CMD_I2C_IO_END: u8 = 1 << 1;   // 0x02
pub const CMD_I2C_IO_READ: u8 = 1 << 3;  // 0x08: Optional flag indicating I2C read

pub const STATUS_IDLE: u8 = 0;
pub const STATUS_ADDRESS_ACK: u8 = 1;
pub const STATUS_ADDRESS_NAK: u8 = 2;

pub const I2C_FUNC_I2C: u32 = 0x0000_0001;
pub const I2C_FUNC_SMBUS_EMUL: u32 = 0x0EFF_0000;
pub const I2C_FUNC_ALL: u32 = I2C_FUNC_I2C | I2C_FUNC_SMBUS_EMUL; // 0x0EFF0001

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum I2cTinyUsbError<E> {
    I2c(E),
    InvalidCommand(u8),
    BufferTooShort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct I2cTinyUsbHandler {
    pub last_status: u8,
    pub delay_us: u16,
}

impl I2cTinyUsbHandler {
    pub const fn new() -> Self {
        Self {
            last_status: STATUS_IDLE,
            delay_us: 0,
        }
    }

    pub fn last_status(&self) -> u8 {
        self.last_status
    }

    pub fn set_last_status(&mut self, status: u8) {
        self.last_status = status;
    }

    pub fn delay_us(&self) -> u16 {
        self.delay_us
    }

    pub fn set_delay_us(&mut self, delay: u16) {
        self.delay_us = delay;
    }

    /// Unified request handler for i2c-tiny-usb protocol.
    ///
    /// For `CMD_I2C_IO`, if `CMD_I2C_IO_READ` is set in `request`, it performs an I2C read into `data`.
    /// Otherwise, it performs an I2C write of `data` to the address in `index & 0x7F`.
    pub fn handle_control_request<I: I2c>(
        &mut self,
        i2c: &mut I,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
    ) -> Result<usize, I2cTinyUsbError<I::Error>> {
        let req_cmd = request;
        let i2c_base = req_cmd & !0x03;

        if i2c_base == CMD_I2C_IO || (req_cmd & 0x04) != 0 {
            let addr = (index & 0x7F) as u8;
            let is_read = (req_cmd & CMD_I2C_IO_READ) != 0;
            if is_read {
                match i2c.read(addr, data) {
                    Ok(()) => {
                        self.last_status = STATUS_ADDRESS_ACK;
                        Ok(data.len())
                    }
                    Err(e) => {
                        self.last_status = STATUS_ADDRESS_NAK;
                        Err(I2cTinyUsbError::I2c(e))
                    }
                }
            } else {
                match i2c.write(addr, data) {
                    Ok(()) => {
                        self.last_status = STATUS_ADDRESS_ACK;
                        Ok(data.len())
                    }
                    Err(e) => {
                        self.last_status = STATUS_ADDRESS_NAK;
                        Err(I2cTinyUsbError::I2c(e))
                    }
                }
            }
        } else {
            match req_cmd {
                CMD_ECHO => {
                    let bytes = value.to_le_bytes();
                    let len = bytes.len().min(data.len());
                    data[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                }
                CMD_GET_FUNC => {
                    let bytes = I2C_FUNC_ALL.to_le_bytes();
                    let len = bytes.len().min(data.len());
                    data[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                }
                CMD_SET_DELAY => {
                    self.delay_us = value;
                    Ok(0)
                }
                CMD_GET_STATUS => {
                    if data.is_empty() {
                        return Err(I2cTinyUsbError::BufferTooShort);
                    }
                    data[0] = self.last_status;
                    Ok(1)
                }
                _ => Err(I2cTinyUsbError::InvalidCommand(request)),
            }
        }
    }

    /// Handle USB Control IN request (data read from device to host).
    pub fn handle_control_in<I: I2c>(
        &mut self,
        i2c: &mut I,
        request: u8,
        value: u16,
        index: u16,
        data: &mut [u8],
    ) -> Result<usize, I2cTinyUsbError<I::Error>> {
        let req_cmd = request;
        let i2c_base = req_cmd & !0x03;

        if i2c_base == CMD_I2C_IO || (req_cmd & 0x04) != 0 {
            let addr = (index & 0x7F) as u8;
            match i2c.read(addr, data) {
                Ok(()) => {
                    self.last_status = STATUS_ADDRESS_ACK;
                    Ok(data.len())
                }
                Err(e) => {
                    self.last_status = STATUS_ADDRESS_NAK;
                    Err(I2cTinyUsbError::I2c(e))
                }
            }
        } else {
            match req_cmd {
                CMD_ECHO => {
                    let bytes = value.to_le_bytes();
                    let len = bytes.len().min(data.len());
                    data[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                }
                CMD_GET_FUNC => {
                    let bytes = I2C_FUNC_ALL.to_le_bytes();
                    let len = bytes.len().min(data.len());
                    data[..len].copy_from_slice(&bytes[..len]);
                    Ok(len)
                }
                CMD_GET_STATUS => {
                    if data.is_empty() {
                        return Err(I2cTinyUsbError::BufferTooShort);
                    }
                    data[0] = self.last_status;
                    Ok(1)
                }
                _ => Err(I2cTinyUsbError::InvalidCommand(request)),
            }
        }
    }

    /// Handle USB Control OUT request (data written from host to device).
    pub fn handle_control_out<I: I2c>(
        &mut self,
        i2c: &mut I,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result<usize, I2cTinyUsbError<I::Error>> {
        let req_cmd = request;
        let i2c_base = req_cmd & !0x03;

        if i2c_base == CMD_I2C_IO || (req_cmd & 0x04) != 0 {
            let addr = (index & 0x7F) as u8;
            match i2c.write(addr, data) {
                Ok(()) => {
                    self.last_status = STATUS_ADDRESS_ACK;
                    Ok(data.len())
                }
                Err(e) => {
                    self.last_status = STATUS_ADDRESS_NAK;
                    Err(I2cTinyUsbError::I2c(e))
                }
            }
        } else {
            match req_cmd {
                CMD_SET_DELAY => {
                    self.delay_us = value;
                    Ok(0)
                }
                _ => Err(I2cTinyUsbError::InvalidCommand(request)),
            }
        }
    }
}
