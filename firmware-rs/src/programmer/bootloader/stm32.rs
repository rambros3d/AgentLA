use crate::programmer::pins::{PIN_UART_RX, PIN_UART_TX};
use rp_pac as pac;

pub const ACK: u8 = 0x79;
pub const NACK: u8 = 0x1F;
pub const SYNC_BYTE: u8 = 0x7F;

pub const CMD_GET: u8 = 0x00;
pub const CMD_GET_VERSION: u8 = 0x01;
pub const CMD_GET_ID: u8 = 0x02;
pub const CMD_READ_MEMORY: u8 = 0x11;
pub const CMD_GO: u8 = 0x21;
pub const CMD_WRITE_MEMORY: u8 = 0x31;
pub const CMD_ERASE: u8 = 0x43;
pub const CMD_EXTENDED_ERASE: u8 = 0x44;
pub const CMD_READOUT_PROTECT: u8 = 0x82;
pub const CMD_READOUT_UNPROTECT: u8 = 0x92;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootloaderError {
    SyncFailed,
    Nack,
    Timeout,
    InvalidResponse,
    ChecksumError,
    PinConflict,
}

pub struct Stm32Bootloader {
    pub tx_pin: usize,
    pub rx_pin: usize,
    pub baud: u32,
}

impl Stm32Bootloader {
    pub fn new(tx_pin: usize, rx_pin: usize, baud: u32) -> Self {
        Self {
            tx_pin,
            rx_pin,
            baud,
        }
    }

    pub fn default_programmer() -> Self {
        Self::new(PIN_UART_TX, PIN_UART_RX, 115_200)
    }

    pub fn init_pins(&self) {
        // TX pin as SIO output, idle high
        pac::IO_BANK0.gpio(self.tx_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::PADS_BANK0.gpio(self.tx_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(false);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << self.tx_pin);
        pac::SIO.gpio_out(0).value_set().write_value(1 << self.tx_pin);

        // RX pin as SIO input with pullup
        pac::IO_BANK0.gpio(self.rx_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::PADS_BANK0.gpio(self.rx_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << self.rx_pin);
    }

    pub fn write_byte_8e1(&self, byte: u8) {
        let bit_cycles = 150_000_000 / self.baud.clamp(1200, 921_600);
        let parity = (byte.count_ones() % 2) != 0;

        // Start bit (low)
        pac::SIO.gpio_out(0).value_clr().write_value(1 << self.tx_pin);
        cortex_m::asm::delay(bit_cycles);

        // 8 Data bits (LSB first)
        for i in 0..8 {
            if (byte & (1 << i)) != 0 {
                pac::SIO.gpio_out(0).value_set().write_value(1 << self.tx_pin);
            } else {
                pac::SIO.gpio_out(0).value_clr().write_value(1 << self.tx_pin);
            }
            cortex_m::asm::delay(bit_cycles);
        }

        // Parity bit (even parity)
        if parity {
            pac::SIO.gpio_out(0).value_set().write_value(1 << self.tx_pin);
        } else {
            pac::SIO.gpio_out(0).value_clr().write_value(1 << self.tx_pin);
        }
        cortex_m::asm::delay(bit_cycles);

        // Stop bit (high)
        pac::SIO.gpio_out(0).value_set().write_value(1 << self.tx_pin);
        cortex_m::asm::delay(bit_cycles);
    }

    pub fn poll_rx_8e1(&self) -> Option<u8> {
        let bit_cycles = 150_000_000 / self.baud.clamp(1200, 921_600);
        if (pac::SIO.gpio_in(0).read() & (1 << self.rx_pin)) != 0 {
            return None;
        }

        // Wait 1.5 bit periods to sample center of bit 0
        cortex_m::asm::delay(bit_cycles + (bit_cycles / 2));

        let mut byte = 0u8;
        for i in 0..8 {
            if (pac::SIO.gpio_in(0).read() & (1 << self.rx_pin)) != 0 {
                byte |= 1 << i;
            }
            cortex_m::asm::delay(bit_cycles);
        }

        // Skip parity bit & wait for stop bit
        cortex_m::asm::delay(bit_cycles);
        cortex_m::asm::delay(bit_cycles / 2);

        Some(byte)
    }

    pub fn read_byte_timeout(&self, timeout_ms: u32) -> Result<u8, BootloaderError> {
        let iterations = timeout_ms * 1000;
        for _ in 0..iterations {
            if let Some(b) = self.poll_rx_8e1() {
                return Ok(b);
            }
            cortex_m::asm::delay(150); // 1 µs
        }
        Err(BootloaderError::Timeout)
    }

    pub fn sync(&mut self) -> Result<(), BootloaderError> {
        // Drain any stale RX
        for _ in 0..10 {
            let _ = self.poll_rx_8e1();
        }
        self.write_byte_8e1(SYNC_BYTE);
        let resp = self.read_byte_timeout(1000)?;
        if resp == ACK {
            Ok(())
        } else if resp == NACK {
            Err(BootloaderError::Nack)
        } else {
            Err(BootloaderError::SyncFailed)
        }
    }

    pub fn send_command(&mut self, cmd: u8) -> Result<(), BootloaderError> {
        self.write_byte_8e1(cmd);
        self.write_byte_8e1(cmd ^ 0xFF);
        let resp = self.read_byte_timeout(1000)?;
        if resp == ACK {
            Ok(())
        } else if resp == NACK {
            Err(BootloaderError::Nack)
        } else {
            Err(BootloaderError::InvalidResponse)
        }
    }

    pub fn readout_unprotect(&mut self) -> Result<(), BootloaderError> {
        self.send_command(CMD_READOUT_UNPROTECT)?;
        // MCU starts automatic mass erase upon first ACK, then sends second ACK
        let resp = self.read_byte_timeout(5000)?;
        if resp == ACK {
            Ok(())
        } else if resp == NACK {
            Err(BootloaderError::Nack)
        } else {
            Err(BootloaderError::InvalidResponse)
        }
    }

    pub fn get_commands(&mut self, supported_cmds: &mut heapless::Vec<u8, 16>) -> Result<u8, BootloaderError> {
        self.send_command(CMD_GET)?;
        let count = self.read_byte_timeout(1000)? as usize;
        let version = self.read_byte_timeout(1000)?;
        supported_cmds.clear();
        for _ in 0..count {
            let cmd = self.read_byte_timeout(1000)?;
            let _ = supported_cmds.push(cmd);
        }
        let ack = self.read_byte_timeout(1000)?;
        if ack != ACK {
            return Err(BootloaderError::InvalidResponse);
        }
        Ok(version)
    }

    pub fn extended_erase_mass(&mut self) -> Result<(), BootloaderError> {
        self.send_command(CMD_EXTENDED_ERASE)?;
        // Special mass erase: 0xFF, 0xFF, Checksum 0x00
        self.write_byte_8e1(0xFF);
        self.write_byte_8e1(0xFF);
        self.write_byte_8e1(0x00);
        let resp = self.read_byte_timeout(5000)?;
        if resp == ACK {
            Ok(())
        } else if resp == NACK {
            Err(BootloaderError::Nack)
        } else {
            Err(BootloaderError::InvalidResponse)
        }
    }

    pub fn write_memory(&mut self, addr: u32, data: &[u8]) -> Result<(), BootloaderError> {
        if data.is_empty() || data.len() > 256 {
            return Err(BootloaderError::InvalidResponse);
        }
        self.send_command(CMD_WRITE_MEMORY)?;

        // Send address [a0, a1, a2, a3, checksum]
        let a0 = ((addr >> 24) & 0xFF) as u8;
        let a1 = ((addr >> 16) & 0xFF) as u8;
        let a2 = ((addr >> 8) & 0xFF) as u8;
        let a3 = (addr & 0xFF) as u8;
        let addr_chk = a0 ^ a1 ^ a2 ^ a3;

        self.write_byte_8e1(a0);
        self.write_byte_8e1(a1);
        self.write_byte_8e1(a2);
        self.write_byte_8e1(a3);
        self.write_byte_8e1(addr_chk);

        let ack = self.read_byte_timeout(1000)?;
        if ack != ACK {
            return Err(BootloaderError::Nack);
        }

        // Send length (N - 1) and data followed by checksum
        let n_minus_1 = (data.len() - 1) as u8;
        let mut chk = n_minus_1;
        self.write_byte_8e1(n_minus_1);
        for &b in data {
            chk ^= b;
            self.write_byte_8e1(b);
        }
        self.write_byte_8e1(chk);

        let ack = self.read_byte_timeout(2000)?;
        if ack == ACK {
            Ok(())
        } else {
            Err(BootloaderError::Nack)
        }
    }
}
