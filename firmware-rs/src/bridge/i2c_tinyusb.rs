#![allow(dead_code)]
//! Native Linux i2c-tiny-usb protocol implementation.
//!
//! Provides driverless USB-to-I2C bus adapter conforming to the Linux kernel
//! `i2c-tiny-usb` protocol (drivers/i2c/busses/i2c-tiny-usb.c).
//! Connects vendor control transfers on EP0 to AgentLA's hardware I2C bus master.

pub use agentla_bridges::i2c_tinyusb::{
    CMD_ECHO, CMD_GET_FUNC, CMD_GET_STATUS, CMD_I2C_IO, CMD_I2C_IO_BEGIN, CMD_I2C_IO_END,
    CMD_I2C_IO_READ, CMD_SET_DELAY, I2C_FUNC_ALL, I2C_FUNC_I2C, I2C_FUNC_SMBUS_EMUL,
    STATUS_ADDRESS_ACK, STATUS_ADDRESS_NAK, STATUS_IDLE, I2cTinyUsbError, I2cTinyUsbHandler,
};

use core::sync::atomic::{AtomicU8, Ordering};
use crate::bridge::i2c::{
    hw_i2c_init, hw_i2c_read_byte, hw_i2c_start, hw_i2c_stop, hw_i2c_write_byte, I2C_PINS,
};

static LAST_STATUS: AtomicU8 = AtomicU8::new(STATUS_IDLE);

pub fn init() {
    LAST_STATUS.store(STATUS_IDLE, Ordering::Relaxed);
    hw_i2c_init(I2C_PINS[0], I2C_PINS[1]);
}

pub fn get_last_status() -> u8 {
    LAST_STATUS.load(Ordering::Relaxed)
}

pub fn set_last_status(status: u8) {
    LAST_STATUS.store(status, Ordering::Relaxed);
}

pub fn i2c_io_read(addr: u8, dest: &mut [u8], is_end: bool) -> bool {
    hw_i2c_init(I2C_PINS[0], I2C_PINS[1]);

    if dest.is_empty() {
        // Zero-byte address probe (scan / detection)
        hw_i2c_start(I2C_PINS[0], I2C_PINS[1]);
        let ack = hw_i2c_write_byte(I2C_PINS[0], I2C_PINS[1], (addr << 1) | 1);
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
        let st = if ack { STATUS_ADDRESS_ACK } else { STATUS_ADDRESS_NAK };
        set_last_status(st);
        return ack;
    }

    hw_i2c_start(I2C_PINS[0], I2C_PINS[1]);
    let ack = hw_i2c_write_byte(I2C_PINS[0], I2C_PINS[1], (addr << 1) | 1);
    if !ack {
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
        dest.fill(0xFF);
        set_last_status(STATUS_ADDRESS_NAK);
        return false;
    }

    let n = dest.len();
    for i in 0..n {
        let ack_slave = i + 1 < n;
        dest[i] = hw_i2c_read_byte(I2C_PINS[0], I2C_PINS[1], ack_slave);
    }

    if is_end {
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
    }

    set_last_status(STATUS_ADDRESS_ACK);
    true
}

pub fn i2c_io_write(addr: u8, data: &[u8], is_end: bool) -> bool {
    hw_i2c_init(I2C_PINS[0], I2C_PINS[1]);

    if data.is_empty() {
        // Zero-byte address probe
        hw_i2c_start(I2C_PINS[0], I2C_PINS[1]);
        let ack = hw_i2c_write_byte(I2C_PINS[0], I2C_PINS[1], addr << 1);
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
        let st = if ack { STATUS_ADDRESS_ACK } else { STATUS_ADDRESS_NAK };
        set_last_status(st);
        return ack;
    }

    hw_i2c_start(I2C_PINS[0], I2C_PINS[1]);
    let ack = hw_i2c_write_byte(I2C_PINS[0], I2C_PINS[1], addr << 1);
    if !ack {
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
        set_last_status(STATUS_ADDRESS_NAK);
        return false;
    }

    let mut ok = true;
    for &b in data {
        if !hw_i2c_write_byte(I2C_PINS[0], I2C_PINS[1], b) {
            ok = false;
            break;
        }
    }

    if is_end || !ok {
        hw_i2c_stop(I2C_PINS[0], I2C_PINS[1]);
    }

    let st = if ok { STATUS_ADDRESS_ACK } else { STATUS_ADDRESS_NAK };
    set_last_status(st);
    ok
}
