#![allow(dead_code, unused)]
//! Hardware & software UART bridge with host CDC-ACM forwarding.
//!
//! Provides bidirectional serial communication between the host PC (via USB CDC-ACM1)
//! and the connected target MCUs (e.g. GPIO 14 telemetry from CH32V003, or GPIO 8/9
//! flexible UART).

use crate::pin_owner;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use rp_pac as pac;

/// Default UART pins: Port 3 telemetry RX on GPIO 14, TX on GPIO 8 (or 15).
pub const PIN_UART_RX_DEFAULT: u32 = 14;
pub const PIN_UART_TX_DEFAULT: u32 = 8;
pub const UART_PINS: [u32; 2] = [PIN_UART_TX_DEFAULT, PIN_UART_RX_DEFAULT];

const RING_CAPACITY: usize = 1024;
struct RingBuffer {
    buf: [u8; RING_CAPACITY],
    head: usize,
    tail: usize,
    count: usize,
}

impl RingBuffer {
    const fn new() -> Self {
        Self {
            buf: [0u8; RING_CAPACITY],
            head: 0,
            tail: 0,
            count: 0,
        }
    }

    fn push(&mut self, byte: u8) -> bool {
        if self.count >= RING_CAPACITY {
            // Overwrite oldest byte
            self.tail = (self.tail + 1) % RING_CAPACITY;
            self.count -= 1;
        }
        self.buf[self.head] = byte;
        self.head = (self.head + 1) % RING_CAPACITY;
        self.count += 1;
        true
    }

    fn pop(&mut self) -> Option<u8> {
        if self.count == 0 {
            None
        } else {
            let byte = self.buf[self.tail];
            self.tail = (self.tail + 1) % RING_CAPACITY;
            self.count -= 1;
            Some(byte)
        }
    }

    fn read(&mut self, dst: &mut [u8]) -> usize {
        let n = dst.len().min(self.count);
        for i in 0..n {
            dst[i] = self.pop().unwrap();
        }
        n
    }
}

static RX_RING: Mutex<CriticalSectionRawMutex, RefCell<RingBuffer>> =
    Mutex::new(RefCell::new(RingBuffer::new()));

pub struct UartBridge {
    active: bool,
    slot: Option<pin_owner::SlotId>,
    pub tx_pin: u32,
    pub rx_pin: u32,
    pub baud: u32,
    pub tx_bytes: u32,
    pub rx_bytes: u32,
}

impl UartBridge {
    pub const fn new() -> Self {
        Self {
            active: false,
            slot: None,
            tx_pin: PIN_UART_TX_DEFAULT,
            rx_pin: PIN_UART_RX_DEFAULT,
            baud: 115_200,
            tx_bytes: 0,
            rx_bytes: 0,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn enable(&mut self) -> Result<(), pin_owner::ClaimError> {
        if self.active {
            return Ok(());
        }
        let pins = [self.tx_pin, self.rx_pin];
        let slot = pin_owner::claim(pin_owner::Driver::Adapter, &pins, pin_owner::CAP_UART)?;
        self.slot = Some(slot);
        self.active = true;

        init_uart_pins(self.tx_pin, self.rx_pin);
        Ok(())
    }

    pub fn disable(&mut self) {
        if let Some(slot) = self.slot.take() {
            let _ = pin_owner::release(slot);
        }
        self.active = false;
    }
}

pub fn init_uart_pins(tx_pin: u32, rx_pin: u32) {
    unsafe {
        // TX pin as SIO output, idle high
        pac::IO_BANK0.gpio(tx_pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(tx_pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(false);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << tx_pin);
        pac::SIO.gpio_out(0).value_set().write_value(1 << tx_pin);

        // RX pin as SIO input with pullup
        pac::IO_BANK0.gpio(rx_pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(rx_pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << rx_pin);
    }
}

fn delay_cycles(cycles: u32) {
    cortex_m::asm::delay(cycles.max(2));
}

/// Push a byte into the global RX ring buffer
pub fn push_rx_byte(byte: u8) {
    RX_RING.lock(|ring| {
        ring.borrow_mut().push(byte);
    });
}

/// Read available bytes from the RX ring buffer
pub fn read_rx_bytes(dst: &mut [u8]) -> usize {
    RX_RING.lock(|ring| {
        ring.borrow_mut().read(dst)
    })
}

/// Number of bytes waiting in the RX ring buffer
pub fn rx_available() -> usize {
    RX_RING.lock(|ring| {
        ring.borrow().count
    })
}

/// Transmit a single UART frame (8N1) out of `tx_pin` at `baud`
pub fn uart_write_byte(tx_pin: u32, baud: u32, byte: u8) {
    let bit_cycles = 150_000_000 / baud.clamp(1200, 921_600);

    unsafe {
        // Start bit (low)
        pac::SIO.gpio_out(0).value_clr().write_value(1 << tx_pin);
        delay_cycles(bit_cycles);

        // 8 Data bits (LSB first)
        for i in 0..8 {
            if (byte & (1 << i)) != 0 {
                pac::SIO.gpio_out(0).value_set().write_value(1 << tx_pin);
            } else {
                pac::SIO.gpio_out(0).value_clr().write_value(1 << tx_pin);
            }
            delay_cycles(bit_cycles);
        }

        // Stop bit (high)
        pac::SIO.gpio_out(0).value_set().write_value(1 << tx_pin);
        delay_cycles(bit_cycles);
    }
}

/// Transmit a buffer of bytes
pub fn uart_write_bytes(tx_pin: u32, baud: u32, data: &[u8]) -> usize {
    for &b in data {
        uart_write_byte(tx_pin, baud, b);
    }
    data.len()
}

/// Non-blocking sample check for incoming start bit on `rx_pin`
pub fn uart_poll_rx(rx_pin: u32, baud: u32) -> Option<u8> {
    let bit_cycles = 150_000_000 / baud.clamp(1200, 921_600);
    unsafe {
        // Check if line is low (start bit)
        if (pac::SIO.gpio_in(0).read() & (1 << rx_pin)) != 0 {
            return None;
        }

        // Wait 1.5 bit periods to sample in the middle of bit 0
        delay_cycles(bit_cycles + (bit_cycles / 2));

        let mut byte = 0u8;
        for i in 0..8 {
            if (pac::SIO.gpio_in(0).read() & (1 << rx_pin)) != 0 {
                byte |= 1 << i;
            }
            delay_cycles(bit_cycles);
        }

        // Wait for stop bit
        delay_cycles(bit_cycles / 2);

        push_rx_byte(byte);
        Some(byte)
    }
}

/// Read UART bytes with a timeout in milliseconds
pub fn uart_read_blocking(rx_pin: u32, baud: u32, dst: &mut [u8], timeout_ms: u32) -> usize {
    let mut count = 0;
    // Check ring buffer first
    count += read_rx_bytes(&mut dst[count..]);
    if count >= dst.len() {
        return count;
    }

    let iterations = timeout_ms * 150_000 / 100;
    for _ in 0..iterations {
        if let Some(b) = uart_poll_rx(rx_pin, baud) {
            dst[count] = b;
            count += 1;
            if count >= dst.len() {
                break;
            }
        }
        delay_cycles(100);
    }
    count
}
