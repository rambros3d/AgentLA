//! RP2350 PIO hardware driver for WCH 2-wire RVSWD.

use super::programs::{
    RVSWD_INSTRUCTIONS, RVSWD_OFFSET_GET_NEXT_CMD, RVSWD_OFFSET_READ_BITS, RVSWD_OFFSET_RESET,
    RVSWD_OFFSET_START, RVSWD_OFFSET_STOP, RVSWD_OFFSET_WRITE_BITS, RVSWD_WRAP_SOURCE,
    RVSWD_WRAP_TARGET,
};
use crate::delay::delay_cycles;
use crate::rvswd::{
    build_header_byte, double_even_parity, header_parity, verify_read_tail, verify_status,
    RvswdError,
};
use crate::traits::RvswdIo;
use rp_pac as pac;

pub struct RvswdTransport {
    prog_offset: u8,
    pub clk_pin: usize,
    pub dio_pin: usize,
    pub dio_in_pin: usize,
    clkdiv: u16,
    initialized: bool,
}

fn pio_set_pindirs(sm: pac::pio::StateMachine, pin: usize, count: usize, is_out: bool) {
    let pinctrl_saved = sm.pinctrl().read();
    let execctrl_saved = sm.execctrl().read();
    sm.execctrl().modify(|w| w.set_out_sticky(false));
    let pindir_val = if is_out { 0x1f } else { 0 };
    sm.pinctrl().write(|w| {
        w.set_set_base(pin as u8);
        w.set_set_count(count as u8);
    });
    let ins = 0b111_00000_100_00000 | (pindir_val & 0x1F);
    sm.instr().write(|w| w.set_instr(ins as u16));
    sm.pinctrl().write_value(pinctrl_saved);
    sm.execctrl().write_value(execctrl_saved);
}

impl Default for RvswdTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl RvswdTransport {
    pub const fn new() -> Self {
        Self {
            prog_offset: 0,
            clk_pin: 8,
            dio_pin: 9,
            dio_in_pin: 9,
            clkdiv: 36,
            initialized: false,
        }
    }

    pub fn init(&mut self) {
        self.init_pins(4, 5);
    }

    pub fn init_pins(&mut self, clk_pin: usize, dio_pin: usize) {
        self.init_pins_split(clk_pin, dio_pin, dio_pin);
    }

    pub fn init_topology(&mut self, topology: crate::traits::ProbeWireTopology) {
        self.init_pins_split(
            topology.clk() as usize,
            topology.dio_out() as usize,
            topology.dio_in() as usize,
        );
    }

    pub fn init_pins_split(&mut self, clk_pin: usize, dio_out_pin: usize, dio_in_pin: usize) {
        self.clk_pin = clk_pin;
        self.dio_pin = dio_out_pin;
        self.dio_in_pin = dio_in_pin;

        pac::PADS_BANK0.gpio(clk_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
            w.set_slewfast(false);
            w.set_drive(pac::pads::vals::Drive::_2M_A);
        });
        pac::IO_BANK0.gpio(clk_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        pac::PADS_BANK0.gpio(dio_out_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
            w.set_slewfast(false);
            w.set_drive(pac::pads::vals::Drive::_2M_A);
        });
        pac::IO_BANK0.gpio(dio_out_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        if dio_in_pin != dio_out_pin {
            pac::PADS_BANK0.gpio(dio_in_pin).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(false);
                w.set_pue(true);
                w.set_pde(false);
                w.set_slewfast(false);
                w.set_drive(pac::pads::vals::Drive::_2M_A);
            });
            pac::IO_BANK0.gpio(dio_in_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
            });
        }

        self.prog_offset = 0;
        for (i, &ins) in RVSWD_INSTRUCTIONS.iter().enumerate() {
            pac::PIO0.instr_mem(self.prog_offset as usize + i).write(|w| {
                w.set_instr_mem(ins);
            });
        }

        let sm = pac::PIO0.sm(0);

        sm.pinctrl().write(|w| {
            w.set_sideset_base(clk_pin as u8);
            w.set_sideset_count(1);
            w.set_out_base(dio_out_pin as u8);
            w.set_out_count(1);
            w.set_set_base(dio_out_pin as u8);
            w.set_set_count(1);
            w.set_in_base(dio_in_pin as u8);
        });

        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + RVSWD_WRAP_TARGET);
            w.set_wrap_top(self.prog_offset + RVSWD_WRAP_SOURCE);
            w.set_side_en(true);
            w.set_side_pindir(false);
        });

        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(false);
            w.set_in_shiftdir(false);
            w.set_autopull(false);
            w.set_autopush(false);
        });

        self.set_clock(6000);

        pio_set_pindirs(sm, clk_pin, 1, true);
        pio_set_pindirs(sm, dio_out_pin, 1, true);
        if dio_in_pin != dio_out_pin {
            pio_set_pindirs(sm, dio_in_pin, 1, false);
        }

        pac::PIO0.fdebug().write(|w| w.0 = 0xFFFFFFFF);

        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset + RVSWD_OFFSET_GET_NEXT_CMD) as u16);
        });

        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(1 << 0);
        });

        self.initialized = true;
        let _ = self.reset();
    }

    pub fn deinit(&mut self) {
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(w.sm_enable() & !(1 << 0));
        });

        let pins = if self.dio_in_pin != self.dio_pin {
            &[self.clk_pin, self.dio_pin, self.dio_in_pin][..]
        } else {
            &[self.clk_pin, self.dio_pin][..]
        };
        for &pin in pins {
            pac::PADS_BANK0.gpio(pin).modify(|w| {
                w.set_pue(false);
                w.set_pde(false);
                w.set_od(true);
                w.set_ie(false);
            });
            pac::IO_BANK0.gpio(pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::NULL.into());
            });
        }
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn set_clock(&mut self, freq_khz: u32) {
        let (int_div, frac_div) = crate::clock::calculate_pio_clkdiv(150_000_000, freq_khz * 1000, 7);
        pac::PIO0.sm(0).clkdiv().write(|w| {
            w.set_int(int_div);
            w.set_frac(frac_div);
        });
        pac::PIO0.sm(1).clkdiv().write(|w| {
            w.set_int(int_div);
            w.set_frac(frac_div);
        });
        self.clkdiv = int_div;
    }

    fn put_fifo(&self, data: u32) -> bool {
        for _ in 0..10_000 {
            if (pac::PIO0.fstat().read().txfull() & (1 << 0)) == 0 {
                pac::PIO0.txf(0).write_value(data);
                return true;
            }
        }
        false
    }

    fn get_fifo(&self) -> Result<u32, RvswdError> {
        for _ in 0..20_000 {
            if (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
                return Ok(pac::PIO0.rxf(0).read());
            }
        }
        Err(RvswdError::Timeout)
    }

    fn send_cmd(&self, cmd_offset: u8, bit_count: u32, out_en: bool) -> bool {
        let count_val = if bit_count == 0 { 0 } else { bit_count - 1 } & 0xFF;
        let full_offset = (self.prog_offset + cmd_offset) as u32;
        let cmd = (count_val << 24) | ((out_en as u32) << 23) | (full_offset << 18);
        self.put_fifo(cmd)
    }

    fn write_bits(&self, data: u32, bit_count: u8) -> bool {
        let shifted = if bit_count < 32 {
            data << (32 - bit_count)
        } else {
            data
        };
        if !self.send_cmd(RVSWD_OFFSET_WRITE_BITS, bit_count as u32, true) {
            return false;
        }
        self.put_fifo(shifted)
    }

    fn read_bits(&self, bit_count: u8) -> Result<u32, RvswdError> {
        if !self.send_cmd(RVSWD_OFFSET_READ_BITS, bit_count as u32, false) {
            return Err(RvswdError::Timeout);
        }
        self.get_fifo()
    }

    fn start_seq(&self) -> bool {
        self.send_cmd(RVSWD_OFFSET_START, 0, true)
    }

    fn stop_seq(&self) -> bool {
        self.send_cmd(RVSWD_OFFSET_STOP, 0, true)
    }

    pub fn reset(&self) -> Result<(), RvswdError> {
        if !self.send_cmd(RVSWD_OFFSET_RESET, 256, true) {
            return Err(RvswdError::Timeout);
        }
        delay_cycles(75_000);

        self.write(0x10, 0x80000001)?;
        self.write(0x10, 0x80000001)?;
        self.write(0x10, 0x00000001)?;
        delay_cycles(750_000);

        Ok(())
    }

    pub fn write(&self, reg: u8, value: u32) -> Result<(), RvswdError> {
        let h = build_header_byte(reg, true);
        let h_par = header_parity(h);
        if !self.start_seq() {
            return Err(RvswdError::Timeout);
        }
        // Header: 8 bits address + rw
        if !self.write_bits(h as u32, 8) {
            return Err(RvswdError::Timeout);
        }
        // Doubled even parity: 2 bits
        if !self.write_bits(h_par as u32, 2) {
            return Err(RvswdError::Timeout);
        }
        // 4 padding zeros
        if !self.write_bits(0, 4) {
            return Err(RvswdError::Timeout);
        }

        // Data: 32 bits
        if !self.write_bits(value, 32) {
            return Err(RvswdError::Timeout);
        }
        // Doubled even parity: 2 bits
        let v_par = double_even_parity(value);
        if !self.write_bits(v_par as u32, 2) {
            return Err(RvswdError::Timeout);
        }

        // Target status: 4 bits
        let st = self.read_bits(4)?;
        if !self.stop_seq() {
            return Err(RvswdError::Timeout);
        }
        verify_status((st & 0x0F) as u8)?;
        Ok(())
    }

    pub fn read(&self, reg: u8) -> Result<u32, RvswdError> {
        let h = build_header_byte(reg, false);
        let h_par = header_parity(h);
        if !self.start_seq() {
            return Err(RvswdError::Timeout);
        }
        // Header: 8 bits address + rw
        if !self.write_bits(h as u32, 8) {
            return Err(RvswdError::Timeout);
        }
        // Doubled even parity: 2 bits
        if !self.write_bits(h_par as u32, 2) {
            return Err(RvswdError::Timeout);
        }
        // 4 padding zeros
        if !self.write_bits(0, 4) {
            return Err(RvswdError::Timeout);
        }

        // Target data: 32 bits
        let data = self.read_bits(32)?;
        // Tail: 6 bits (parity[1], 1, status[4])
        let tail = self.read_bits(6)?;
        if !self.stop_seq() {
            return Err(RvswdError::Timeout);
        }

        verify_read_tail(data, tail as u8)?;
        Ok(data)
    }
}

impl RvswdIo for RvswdTransport {
    type Error = RvswdError;

    fn write(&mut self, reg: u8, value: u32) -> Result<(), Self::Error> {
        RvswdTransport::write(self, reg, value)
    }

    fn read(&mut self, reg: u8) -> Result<u32, Self::Error> {
        RvswdTransport::read(self, reg)
    }

    fn reset(&mut self) -> Result<(), Self::Error> {
        RvswdTransport::reset(self)
    }

    fn set_clock(&mut self, freq_khz: u32) -> Result<(), Self::Error> {
        RvswdTransport::set_clock(self, freq_khz);
        Ok(())
    }
}
