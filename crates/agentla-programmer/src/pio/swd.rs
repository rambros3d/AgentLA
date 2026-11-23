//! RP2350 PIO hardware driver for Arm SWD.

use super::programs::{
    SWD_INSTRUCTIONS, SWD_OFFSET_GET_NEXT_CMD, SWD_OFFSET_READ_CMD,
    SWD_OFFSET_TURNAROUND_CMD, SWD_OFFSET_WRITE_CMD, SWD_WRAP_SOURCE, SWD_WRAP_TARGET,
};
use crate::delay::delay_cycles;
use crate::swd::{
    build_request, Adiv5Access, SwdError, DP_ABORT, DP_CTRLSTAT, DP_DPIDR, DP_RDBUFF,
    DP_SELECT, SWD_ACK_FAULT, SWD_ACK_OK, SWD_ACK_PARITY_ERR, SWD_ACK_WAIT,
};
use crate::traits::SwdIo;
use rp_pac as pac;

#[derive(Clone, Copy)]
enum ProbeCmd {
    Write,
    #[allow(dead_code)]
    Skip,
    Turnaround,
    Read,
}

pub struct SwdTransport {
    prog_offset: u8,
    pub clk_pin: usize,
    pub dio_pin: usize,
    pub dio_in_pin: usize,
    pub reset_pin: usize,
    turnaround: u8,
    idle_cycles: u8,
    ap_select: u32,
    clock_khz: u32,
    pub initialized: bool,
    pub reset_asserted: bool,
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

impl Default for SwdTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SwdTransport {
    pub const fn new() -> Self {
        Self {
            prog_offset: 0,
            clk_pin: 4,
            dio_pin: 5,
            dio_in_pin: 5,
            reset_pin: 6,
            turnaround: 1,
            idle_cycles: 8,
            ap_select: 0xFFFFFFFF,
            clock_khz: 1000,
            initialized: false,
            reset_asserted: false,
        }
    }

    pub fn restore_pio_pins(&self) {
        pac::PADS_BANK0.gpio(self.clk_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
        });
        pac::PADS_BANK0.gpio(self.dio_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
        });
        if self.dio_in_pin != self.dio_pin {
            pac::PADS_BANK0.gpio(self.dio_in_pin).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
            });
            pac::IO_BANK0.gpio(self.dio_in_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
            });
        }
        pac::IO_BANK0.gpio(self.clk_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });
        pac::IO_BANK0.gpio(self.dio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });
        let mut oe_clr = (1 << self.clk_pin) | (1 << self.dio_pin);
        if self.dio_in_pin != self.dio_pin {
            oe_clr |= 1 << self.dio_in_pin;
        }
        pac::SIO.gpio_oe(0).value_clr().write_value(oe_clr);
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(w.sm_enable() | (1 << 0));
        });
    }

    pub fn ensure_initialized(&mut self) {
        let sm_enabled = (pac::PIO0.ctrl().read().sm_enable() & (1 << 0)) != 0;
        let func_clk = pac::IO_BANK0.gpio(self.clk_pin).ctrl().read().funcsel();
        if !self.initialized || !sm_enabled || func_clk != pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into() {
            self.init_pins_split(self.clk_pin, self.dio_pin, self.dio_in_pin, self.reset_pin, self.clock_khz);
        } else {
            self.restore_pio_pins();
        }
    }

    pub fn init(&mut self, freq_khz: u32) {
        self.init_pins(4, 5, 6, freq_khz);
    }

    pub fn init_pins(&mut self, clk_pin: usize, dio_pin: usize, reset_pin: usize, freq_khz: u32) {
        self.init_pins_split(clk_pin, dio_pin, dio_pin, reset_pin, freq_khz);
    }

    pub fn init_topology(&mut self, topology: crate::traits::ProbeWireTopology, reset_pin: usize, freq_khz: u32) {
        self.init_pins_split(
            topology.clk() as usize,
            topology.dio_out() as usize,
            topology.dio_in() as usize,
            reset_pin,
            freq_khz,
        );
    }

    pub fn init_pins_split(
        &mut self,
        clk_pin: usize,
        dio_out_pin: usize,
        dio_in_pin: usize,
        reset_pin: usize,
        freq_khz: u32,
    ) {
        self.clk_pin = clk_pin;
        self.dio_pin = dio_out_pin;
        self.dio_in_pin = dio_in_pin;
        self.reset_pin = reset_pin;
        self.initialized = true;

        // 1. Configure GPIO pads
        pac::PADS_BANK0.gpio(clk_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
            w.set_slewfast(false);
            w.set_drive(pac::pads::vals::Drive::_4M_A);
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
            w.set_drive(pac::pads::vals::Drive::_4M_A);
        });
        pac::IO_BANK0.gpio(dio_out_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        if dio_in_pin != dio_out_pin {
            pac::PADS_BANK0.gpio(dio_in_pin).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
                w.set_pde(false);
                w.set_slewfast(false);
                w.set_drive(pac::pads::vals::Drive::_4M_A);
            });
            pac::IO_BANK0.gpio(dio_in_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
            });
        }

        if !self.reset_asserted {
            pac::PADS_BANK0.gpio(reset_pin).modify(|w| {
                w.set_iso(false);
                w.set_ie(true);
                w.set_od(true);
                w.set_pue(true);
                w.set_pde(false);
            });
            pac::IO_BANK0.gpio(reset_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::NULL.into());
            });
        }

        // 2. Load PIO Program into PIO0
        self.prog_offset = 0;
        for (i, &ins) in SWD_INSTRUCTIONS.iter().enumerate() {
            pac::PIO0.instr_mem(self.prog_offset as usize + i).write(|w| {
                w.set_instr_mem(ins);
            });
        }

        // 3. Configure State Machine 0
        let sm = pac::PIO0.sm(0);

        sm.pinctrl().write(|w| {
            w.set_sideset_base(clk_pin as u8);
            // With side_en=true (optional side-set), the side-set field width in PINCTRL must be 2:
            // 1 bit for the side-set enable flag (MSB), and 1 bit for the SWCLK GPIO value.
            // This drives exactly 1 pin (clk_pin) and leaves 3 bits (10:8) for instruction delay.
            w.set_sideset_count(2);
            w.set_out_base(dio_out_pin as u8);
            w.set_out_count(1);
            w.set_set_base(dio_out_pin as u8);
            w.set_set_count(1);
            w.set_in_base(dio_in_pin as u8);
        });

        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + SWD_WRAP_TARGET);
            w.set_wrap_top(self.prog_offset + SWD_WRAP_SOURCE);
            w.set_side_en(true);
            w.set_side_pindir(false);
        });

        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(true);
            w.set_in_shiftdir(true);
            w.set_autopull(false);
            w.set_autopush(false);
        });

        pio_set_pindirs(sm, clk_pin, 1, true);
        pio_set_pindirs(sm, dio_out_pin, 1, false);
        if dio_in_pin != dio_out_pin {
            pio_set_pindirs(sm, dio_in_pin, 1, false);
        }

        self.set_clock(freq_khz);

        sm.instr().write(|w| w.set_instr(0x80a0));
        pac::PIO0.fdebug().write(|w| w.0 = 0xFFFFFFFF);

        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset + SWD_OFFSET_GET_NEXT_CMD) as u16);
        });

        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(1 << 0);
        });

        self.ap_select = 0xFFFFFFFF;
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

    pub fn set_clock(&mut self, freq_khz: u32) {
        let (int_div, frac_div) = crate::clock::calculate_pio_clkdiv(150_000_000, freq_khz * 1000, 4);
        pac::PIO0.sm(0).clkdiv().write(|w| {
            w.set_int(int_div);
            w.set_frac(frac_div);
        });
        self.clock_khz = freq_khz;
    }

    pub fn set_turnaround(&mut self, cycles: u8) {
        self.turnaround = if cycles == 0 { 1 } else { cycles };
    }

    pub fn set_idle_cycles(&mut self, cycles: u8) {
        self.idle_cycles = cycles;
    }

    #[inline(always)]
    fn fmt_command(&self, bit_count: u32, out_en: bool, cmd: ProbeCmd) -> u32 {
        let cmd_addr = match cmd {
            ProbeCmd::Write => self.prog_offset + SWD_OFFSET_WRITE_CMD,
            ProbeCmd::Skip => self.prog_offset + SWD_OFFSET_GET_NEXT_CMD,
            ProbeCmd::Turnaround => self.prog_offset + SWD_OFFSET_TURNAROUND_CMD,
            ProbeCmd::Read => self.prog_offset + SWD_OFFSET_READ_CMD,
        };
        let count = if bit_count == 0 { 0 } else { bit_count - 1 } & 0xFF;
        count | ((out_en as u32) << 8) | ((cmd_addr as u32) << 9)
    }

    fn put_fifo(&self, data: u32) -> bool {
        for _ in 0..2_000_000 {
            if (pac::PIO0.fstat().read().txfull() & (1 << 0)) == 0 {
                pac::PIO0.txf(0).write_value(data);
                return true;
            }
        }
        false
    }

    pub fn write_bits_raw(&self, bit_count: u32, data: u32) {
        if bit_count == 0 {
            return;
        }
        if !self.put_fifo(self.fmt_command(bit_count, true, ProbeCmd::Write)) {
            return;
        }
        self.put_fifo(data);
    }

    pub fn write_bits(&self, bit_count: u32, data: u32) {
        self.write_bits_raw(bit_count, data);
    }

    pub fn hiz_clocks(&self, bit_count: u32) {
        if bit_count == 0 {
            return;
        }
        if !self.put_fifo(self.fmt_command(bit_count, false, ProbeCmd::Turnaround)) {
            return;
        }
        self.put_fifo(0);
    }

    pub fn read_bits(&self, bit_count: u32) -> u32 {
        if bit_count == 0 {
            return 0;
        }
        if !self.put_fifo(self.fmt_command(bit_count, false, ProbeCmd::Read)) {
            return 0xEEEEEEEE;
        }

        for _ in 0..2_000_000 {
            if (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
                let data = pac::PIO0.rxf(0).read();
                return if bit_count < 32 {
                    data >> (32 - bit_count)
                } else {
                    data
                };
            }
        }
        0xDDDDDDDD
    }

    pub fn wait_idle(&self) {
        pac::PIO0.fdebug().write(|w| w.set_txstall(1 << 0));
        for _ in 0..2_000_000 {
            if (pac::PIO0.fdebug().read().txstall() & (1 << 0)) != 0 {
                break;
            }
        }
    }

    pub fn line_reset(&mut self) {
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);
        self.write_bits_raw(8, 0x00000000);
        let _ = self.wait_idle();
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn jtag_to_swd(&mut self) {
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(24, 0x00FFFFFF);
        self.write_bits_raw(16, 0xE79E);
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);
        self.write_bits_raw(8, 0x00);
        let _ = self.wait_idle();
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn dormant_to_swd(&mut self) {
        // RP2350 / CoreSight ADIv6 Dormant-to-SWD wake sequence:
        // 1. At least 8 cycles SWCLK HIGH (send 16 cycles)
        self.write_bits_raw(16, 0xFFFF);

        // 2. 128-bit Selection Alert sequence (LSB-first):
        // Word 0: 0x19BC0EA2
        // Word 1: 0xE3DDAFE9
        // Word 2: 0x86852D95
        // Word 3: 0x6209F392
        self.write_bits_raw(32, 0x19BC0EA2);
        self.write_bits_raw(32, 0xE3DDAFE9);
        self.write_bits_raw(32, 0x86852D95);
        self.write_bits_raw(32, 0x6209F392);

        // 3. 4 cycles LOW followed by 8-bit activation code 0x1A (ARM SWD DP)
        // Combined 12-bit sequence: (0x1A << 4) = 0x01A0 (LSB-first)
        self.write_bits_raw(12, 0x01A0);

        // 4. Line reset (at least 50 cycles HIGH) and 8 idle cycles LOW
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);
        self.write_bits_raw(8, 0x00);
        let _ = self.wait_idle();
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn dormant_to_swd_reversed(&mut self) {
        self.write_bits_raw(16, 0xFFFF);
        self.write_bits_raw(32, 0x6209F392);
        self.write_bits_raw(32, 0x86852D95);
        self.write_bits_raw(32, 0xE3DDAFE9);
        self.write_bits_raw(32, 0x19BC0EA2);
        self.write_bits_raw(12, 0x01A0);
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);
        self.write_bits_raw(8, 0x00);
        let _ = self.wait_idle();
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn jtag_to_dormant_to_swd(&mut self) {
        // Line reset (60 cycles HIGH)
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);

        // JTAG to Dormant sequence (ARM ADIv6 B5.3.2):
        // 5 cycles TMS HIGH (0x1F)
        // 31-bit sequence 0x33BBBBBA (LSB-first)
        // 8 cycles TMS HIGH (0xFF)
        self.write_bits_raw(5, 0x1F);
        self.write_bits_raw(31, 0x33BBBBBA);
        self.write_bits_raw(8, 0xFF);

        // 128-bit Selection Alert sequence (LSB-first)
        self.write_bits_raw(32, 0x19BC0EA2);
        self.write_bits_raw(32, 0xE3DDAFE9);
        self.write_bits_raw(32, 0x86852D95);
        self.write_bits_raw(32, 0x6209F392);

        // 4 cycles LOW + 8-bit activation code 0x1A (ARM SWD DP)
        self.write_bits_raw(12, 0x01A0);

        // Post-activation line reset (60 cycles HIGH) and 8 idle cycles LOW
        self.write_bits_raw(32, 0xFFFFFFFF);
        self.write_bits_raw(28, 0x0FFFFFFF);
        self.write_bits_raw(8, 0x00);
        let _ = self.wait_idle();
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn drain_rx_fifo(&self) {
        while (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
            let _ = pac::PIO0.rxf(0).read();
        }
    }

    pub fn transfer_raw(&self, request: u8, data: &mut u32) -> u8 {
        self.drain_rx_fifo();
        self.write_bits(8, request as u32);
        let is_read = (request & (1 << 2)) != 0;
        let ack_cycles = if is_read {
            (self.turnaround + 3) as u32
        } else {
            (self.turnaround + 3 + self.turnaround) as u32
        };
        let raw_ack = self.read_bits(ack_cycles);
        let ack = (raw_ack >> self.turnaround) as u8 & 0x07;

        if ack == SWD_ACK_OK {
            if is_read {
                let val = self.read_bits(32);
                let bit = self.read_bits(1) as u8;
                let parity = (val.count_ones() & 1) as u8;
                self.hiz_clocks(self.turnaround as u32);
                if parity != bit {
                    return SWD_ACK_PARITY_ERR;
                }
                *data = val;
            } else {
                // Turnaround clock cycle was clocked by ack_cycles above with SWDIO as input.
                // Target has completed releasing SWDIO with SWCLK low.
                // Write 32-bit WDATA followed by 1-bit parity.
                let val = *data;
                self.write_bits(32, val);
                let parity = (val.count_ones() & 1) as u32;
                self.write_bits(1, parity);
            }

            let idle = core::cmp::max(self.idle_cycles, 2);
            self.write_bits(idle as u32, 0);
            let _ = self.wait_idle();
            return SWD_ACK_OK;
        }

        if ack == SWD_ACK_WAIT || ack == SWD_ACK_FAULT {
            if is_read {
                self.hiz_clocks(self.turnaround as u32);
            }
            let idle = core::cmp::max(self.idle_cycles, 8);
            self.write_bits(idle as u32, 0);
            let _ = self.wait_idle();
            return ack;
        }

        self.read_bits((self.turnaround as u32) + 33);
        let _ = self.wait_idle();
        ack
    }

    pub fn transfer(&self, request: u8, data: &mut u32) -> u8 {
        self.transfer_raw(request, data)
    }

    pub fn build_request(is_ap: bool, is_read: bool, a23: u8) -> u8 {
        build_request(is_ap, is_read, a23)
    }

    pub fn write_targetsel(&mut self, targetid: u32) {
        let req = 0x99u8;
        self.drain_rx_fifo();
        let _ = self.write_bits(8, req as u32);
        // Trn (1) + ACK (3) + Trn (1) = 5 cycles with pins Hi-Z
        self.hiz_clocks(5);
        // 32-bit targetid data (LSB first)
        let _ = self.write_bits(32, targetid);
        // 1-bit parity
        let parity = (targetid.count_ones() & 1) as u32;
        let _ = self.write_bits(1, parity);
        // Idle cycles
        let _ = self.write_bits(8, 0);
    }

    pub fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        let req = Self::build_request(false, false, (reg >> 2) & 3);
        let mut val = value;
        for _ in 0..200 {
            let ack = self.transfer_raw(req, &mut val);
            crate::stlink::stlink_diag::set_last_ack(ack, reg);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                delay_cycles(300);
                continue;
            } else if ack == SWD_ACK_FAULT {
                let req_abort = Self::build_request(false, false, 0);
                let mut val_abort = 0x0000001Eu32;
                let _ = self.transfer_raw(req_abort, &mut val_abort);
                delay_cycles(300);
                continue;
            } else {
                return Err(SwdError::ProtocolError);
            }
        }
        Err(SwdError::Timeout)
    }

    pub fn read_dp(&self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let req = Self::build_request(false, true, (reg >> 2) & 3);
        for _ in 0..200 {
            let ack = self.transfer_raw(req, value);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                delay_cycles(300);
                continue;
            } else if ack == SWD_ACK_FAULT {
                let req_abort = Self::build_request(false, false, 0);
                let mut val_abort = 0x0000001Eu32;
                let _ = self.transfer_raw(req_abort, &mut val_abort);
                return Err(SwdError::AckFault);
            } else if ack == SWD_ACK_PARITY_ERR {
                return Err(SwdError::ParityError);
            } else {
                return Err(SwdError::ProtocolError);
            }
        }
        Err(SwdError::Timeout)
    }

    pub fn select_ap(&mut self, apsel: u8, apbank: u8) -> Result<(), SwdError> {
        let select = ((apsel as u32) << 24) | (((apbank as u32) & 0x0F) << 4);
        if self.ap_select != select {
            self.write_dp(DP_SELECT, select)?;
            self.ap_select = select;
        }
        Ok(())
    }

    pub fn power_up_debug(&mut self) -> Result<(), SwdError> {
        let s1 = self.write_dp(DP_ABORT, 0x0000001E);
        let s2 = self.write_dp(DP_CTRLSTAT, 0x50000000);
        crate::stlink::stlink_diag::set_writes(
            ((s1.is_ok() as u32) << 1) | ((s2.is_ok() as u32) << 2),
        );
        let mut last = 0u32;
        let mut last_st = 0u32;
        for _ in 0..200 {
            let mut val = 0u32;
            if self.read_dp(DP_CTRLSTAT, &mut val).is_ok()
                && (val & 0xA0000000) == 0xA0000000
            {
                let _ = self.write_dp(DP_SELECT, 0);
                self.ap_select = 0;
                return Ok(());
            }
            last = val;
            delay_cycles(1500);
        }
        crate::stlink::stlink_diag::set_ctrlstat(last);
        Err(SwdError::Timeout)
    }

    pub fn recover_from_ack_error(&mut self) -> Result<(), SwdError> {
        self.jtag_to_swd();
        let mut idcode = 0u32;
        if self.read_dpidr_retry(&mut idcode).is_err() || idcode == 0 || idcode == 0xFFFFFFFF {
            return Err(SwdError::ProtocolError);
        }
        let _ = self.write_dp(DP_ABORT, 0x0000001E);
        let _ = self.write_dp(DP_CTRLSTAT, 0x50000000);
        self.power_up_debug()?;
        self.ap_select = 0xFFFFFFFF;
        Ok(())
    }

    pub fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        let apsel = if self.ap_select == 0xFFFFFFFF { 0 } else { (self.ap_select >> 24) as u8 };
        self.write_ap_port(apsel, reg, value)
    }

    pub fn write_ap_port(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        match self.raw_write_ap_no_recovery(apsel, reg, value) {
            Ok(()) => Ok(()),
            Err(_) => {
                if self.recover_from_ack_error().is_ok() {
                    self.raw_write_ap_no_recovery(apsel, reg, value)
                } else {
                    Err(SwdError::ProtocolError)
                }
            }
        }
    }

    pub fn raw_write_ap_no_recovery(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        let apbank = (reg >> 4) & 0x0F;
        self.select_ap(apsel, apbank)?;
        let req = Self::build_request(true, false, (reg >> 2) & 3);
        let mut val = value;
        for _ in 0..5000 {
            let ack = self.transfer_raw(req, &mut val);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                delay_cycles(1500);
                continue;
            } else if ack == SWD_ACK_FAULT {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                delay_cycles(1500);
                continue;
            } else {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                return Err(SwdError::ProtocolError);
            }
        }
        let _ = self.write_dp(DP_ABORT, 0x0000001E);
        Err(SwdError::Timeout)
    }

    pub fn read_ap(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let apsel = if self.ap_select == 0xFFFFFFFF { 0 } else { (self.ap_select >> 24) as u8 };
        self.read_ap_port(apsel, reg, value)
    }

    pub fn read_ap_port(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        match self.raw_read_ap_no_recovery(apsel, reg, value) {
            Ok(()) => Ok(()),
            Err(_) => {
                if self.recover_from_ack_error().is_ok() {
                    self.raw_read_ap_no_recovery(apsel, reg, value)
                } else {
                    Err(SwdError::ProtocolError)
                }
            }
        }
    }

    pub fn raw_read_ap_no_recovery(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let apbank = (reg >> 4) & 0x0F;
        self.select_ap(apsel, apbank)?;
        let req = Self::build_request(true, true, (reg >> 2) & 3);
        let mut dummy = 0u32;
        for _ in 0..2000 {
            let ack = self.transfer_raw(req, &mut dummy);
            if ack == SWD_ACK_OK {
                let res = self.read_dp(DP_RDBUFF, value);
                if res.is_err() {
                    let _ = self.write_dp(DP_ABORT, 0x0000001E);
                }
                return res;
            } else if ack == SWD_ACK_WAIT {
                delay_cycles(1500);
                continue;
            } else if ack == SWD_ACK_FAULT {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                delay_cycles(1500);
                continue;
            } else {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                return Err(SwdError::ProtocolError);
            }
        }
        let _ = self.write_dp(DP_ABORT, 0x0000001E);
        Err(SwdError::Timeout)
    }

    pub fn mem_read_chunk(&mut self, apsel: u8, addr: u32, words: &mut [u32]) -> Result<(), SwdError> {
        if words.is_empty() {
            return Ok(());
        }
        self.write_ap_port(apsel, crate::swd::AP_TAR, addr)?;
        let drw_read_req = Self::build_request(true, true, (crate::swd::AP_DRW >> 2) & 3);
        let rdbuff_read_req = Self::build_request(false, true, (crate::swd::DP_RDBUFF >> 2) & 3);

        let mut dummy = 0u32;
        let mut ok = false;
        for _ in 0..200 {
            let ack = self.transfer_raw(drw_read_req, &mut dummy);
            if ack == SWD_ACK_OK {
                ok = true;
                break;
            } else if ack == SWD_ACK_WAIT {
                delay_cycles(1500);
                continue;
            } else if ack == SWD_ACK_FAULT {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                delay_cycles(1500);
                continue;
            } else {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                return Err(SwdError::ProtocolError);
            }
        }
        if !ok {
            let _ = self.write_dp(DP_ABORT, 0x0000001E);
            return Err(SwdError::Timeout);
        }

        for i in 0..words.len() {
            let last = i + 1 == words.len();
            let req = if last { rdbuff_read_req } else { drw_read_req };
            let mut word_ok = false;
            for _ in 0..200 {
                let ack = self.transfer_raw(req, &mut words[i]);
                if ack == SWD_ACK_OK {
                    word_ok = true;
                    break;
                } else if ack == SWD_ACK_WAIT {
                    delay_cycles(1500);
                    continue;
                } else if ack == SWD_ACK_FAULT {
                    let _ = self.write_dp(DP_ABORT, 0x0000001E);
                    delay_cycles(1500);
                    continue;
                } else {
                    let _ = self.write_dp(DP_ABORT, 0x0000001E);
                    return Err(SwdError::ProtocolError);
                }
            }
            if !word_ok {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                return Err(SwdError::Timeout);
            }
        }
        Ok(())
    }

    pub fn mem_write_chunk(&mut self, apsel: u8, addr: u32, words: &[u32]) -> Result<(), SwdError> {
        if words.is_empty() {
            return Ok(());
        }
        self.write_ap_port(apsel, crate::swd::AP_TAR, addr)?;
        let drw_write_req = Self::build_request(true, false, (crate::swd::AP_DRW >> 2) & 3);

        for &val in words {
            let mut word_val = val;
            let mut word_ok = false;
            for _ in 0..5000 {
                let ack = self.transfer_raw(drw_write_req, &mut word_val);
                if ack == SWD_ACK_OK {
                    word_ok = true;
                    break;
                } else if ack == SWD_ACK_WAIT {
                    delay_cycles(1500);
                    continue;
                } else if ack == SWD_ACK_FAULT {
                    let _ = self.write_dp(DP_ABORT, 0x0000001E);
                    delay_cycles(1500);
                    continue;
                } else {
                    let _ = self.write_dp(DP_ABORT, 0x0000001E);
                    return Err(SwdError::ProtocolError);
                }
            }
            if !word_ok {
                let _ = self.write_dp(DP_ABORT, 0x0000001E);
                return Err(SwdError::Timeout);
            }
        }

        Ok(())
    }

    pub fn assert_reset(&mut self, assert: bool) {
        self.reset_asserted = assert;
        self.ap_select = 0xFFFFFFFF;
        pac::PADS_BANK0.gpio(self.reset_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::IO_BANK0.gpio(self.reset_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::SIO.gpio_oe(0).value_set().write_value(1 << self.reset_pin);
        if assert {
            pac::SIO.gpio_out(0).value_clr().write_value(1 << self.reset_pin);
        } else {
            pac::SIO.gpio_out(0).value_set().write_value(1 << self.reset_pin);
        }
    }

    pub fn read_gpio_levels(&self) -> u8 {
        let val = pac::SIO.gpio_in(0).read();
        let mut res = 0u8;
        if (val & (1 << self.clk_pin)) != 0 {
            res |= 1;
        }
        if (val & (1 << self.dio_in_pin)) != 0 {
            res |= 2;
        }
        if (val & (1 << self.reset_pin)) != 0 {
            res |= 4;
        }
        res
    }

    pub fn attach_targetsel(&mut self, targetid: u32, idcode: &mut u32) -> bool {
        self.jtag_to_swd();
        delay_cycles(1500);
        self.write_targetsel(targetid);
        delay_cycles(1500);
        let _ = self.read_dp(DP_DPIDR, idcode);
        self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF
    }

    pub fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError> {
        let freqs = [4000, 2000, 1000, 500, 100];
        for &freq in &freqs {
            self.set_clock(freq);

            // 1. Legacy JTAG-to-SWD (STM32H5, STM32F1/F4)
            self.jtag_to_swd();
            delay_cycles(15_000);
            let _ = self.read_dp(DP_DPIDR, idcode);
            if self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }

            // 2. JTAG-to-Dormant-to-SWD (ARM ADIv6 B5.3.2)
            self.jtag_to_dormant_to_swd();
            delay_cycles(15_000);
            let _ = self.read_dp(DP_DPIDR, idcode);
            if self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }

            // 3. RP2350 ADIv6 Pure Dormant-to-SWD (canonical alert order)
            self.dormant_to_swd();
            delay_cycles(15_000);
            let _ = self.read_dp(DP_DPIDR, idcode);
            if self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }

            // 4. RP2350 ADIv6 Pure Dormant-to-SWD (reversed alert word order)
            self.dormant_to_swd_reversed();
            delay_cycles(15_000);
            let _ = self.read_dp(DP_DPIDR, idcode);
            if self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }

            // 5. Plain Line Reset (if already in SWD)
            self.line_reset();
            delay_cycles(15_000);
            let _ = self.read_dp(DP_DPIDR, idcode);
            if self.read_dp(DP_DPIDR, idcode).is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }
        }
        Err(SwdError::ProtocolError)
    }

    pub fn try_attach(&mut self) -> Result<u32, SwdError> {
        let mut dpidr = 0u32;
        let orig_clk = self.clk_pin;
        let orig_dio = self.dio_pin;
        let rst = self.reset_pin;

        // Pass 1: Standard pin configuration (GP4=CLK, GP5=DIO) normal attach
        self.init_pins(orig_clk, orig_dio, rst, 100);
        self.assert_reset(false);
        if self.read_dpidr_retry(&mut dpidr).is_ok() && dpidr != 0 && dpidr != 0xFFFFFFFF {
            return Ok(dpidr);
        }

        // Pass 2: Standard pins WITH Connect-Under-Reset (holding NRST low)
        self.assert_reset(true);
        delay_cycles(300_000); // 2ms in reset
        let res_rst = self.read_dpidr_retry(&mut dpidr);
        if res_rst.is_ok() && dpidr != 0 && dpidr != 0xFFFFFFFF {
            self.assert_reset(false);
            delay_cycles(150_000);
            return Ok(dpidr);
        }
        self.assert_reset(false);
        delay_cycles(300_000);

        // Pass 3: Swapped pin configuration (GP5=CLK, GP4=DIO) in case wiring is reversed
        self.init_pins(orig_dio, orig_clk, rst, 100);
        self.assert_reset(false);
        if self.read_dpidr_retry(&mut dpidr).is_ok() && dpidr != 0 && dpidr != 0xFFFFFFFF {
            return Ok(dpidr);
        }

        // Pass 4: Swapped pins WITH Connect-Under-Reset
        self.assert_reset(true);
        delay_cycles(300_000);
        let res_swapped_rst = self.read_dpidr_retry(&mut dpidr);
        if res_swapped_rst.is_ok() && dpidr != 0 && dpidr != 0xFFFFFFFF {
            self.assert_reset(false);
            delay_cycles(150_000);
            return Ok(dpidr);
        }
        self.assert_reset(false);
        delay_cycles(300_000);

        // Restore original pins
        self.init_pins(orig_clk, orig_dio, rst, 100);
        self.assert_reset(false);
        Err(SwdError::ProtocolError)
    }

    pub fn bitbang_attach(&mut self) -> Result<u32, (u8, u32)> {
        self.try_attach().map_err(|_| (7, 0))
    }
}

impl SwdIo for SwdTransport {
    type Error = SwdError;

    fn ensure_initialized(&mut self) -> Result<(), Self::Error> {
        self.ensure_initialized();
        Ok(())
    }

    fn wait_idle(&mut self) -> Result<(), Self::Error> {
        SwdTransport::wait_idle(self);
        Ok(())
    }

    fn transfer(&mut self, req: u8, data: Option<u32>) -> Result<(u8, u32), Self::Error> {
        self.ensure_initialized();
        let mut val = data.unwrap_or(0);
        let ack = self.transfer_raw(req, &mut val);
        Ok((ack, val))
    }

    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        self.line_reset();
        Ok(())
    }

    fn write_bits(&mut self, bit_count: u32, data: u32) -> Result<(), Self::Error> {
        SwdTransport::write_bits(self, bit_count, data);
        Ok(())
    }

    fn read_bits(&mut self, bit_count: u32) -> Result<u32, Self::Error> {
        Ok(SwdTransport::read_bits(self, bit_count))
    }

    fn jtag_to_swd(&mut self) -> Result<(), Self::Error> {
        SwdTransport::jtag_to_swd(self);
        Ok(())
    }

    fn dormant_to_swd(&mut self) -> Result<(), Self::Error> {
        SwdTransport::dormant_to_swd(self);
        Ok(())
    }

    fn jtag_to_dormant_to_swd(&mut self) -> Result<(), Self::Error> {
        SwdTransport::jtag_to_dormant_to_swd(self);
        Ok(())
    }

    fn set_clock(&mut self, freq_khz: u32) -> Result<(), Self::Error> {
        SwdTransport::set_clock(self, freq_khz);
        Ok(())
    }

    fn set_turnaround(&mut self, cycles: u8) {
        SwdTransport::set_turnaround(self, cycles);
    }

    fn set_idle_cycles(&mut self, cycles: u8) {
        SwdTransport::set_idle_cycles(self, cycles);
    }

    fn assert_reset(&mut self, assert: bool) -> Result<(), Self::Error> {
        self.assert_reset(assert);
        Ok(())
    }

    fn swj_pins(
        &mut self,
        pin_output: u8,
        pin_select: u8,
        wait_us: u32,
    ) -> Result<u8, Self::Error> {
        if (pin_select & 0x01) != 0 {
            pac::IO_BANK0.gpio(self.clk_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << self.clk_pin);
            if (pin_output & 0x01) != 0 {
                pac::SIO.gpio_out(0).value_set().write_value(1 << self.clk_pin);
            } else {
                pac::SIO.gpio_out(0).value_clr().write_value(1 << self.clk_pin);
            }
        }

        if (pin_select & 0x02) != 0 {
            pac::IO_BANK0.gpio(self.dio_pin).ctrl().write(|w| {
                w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
            });
            pac::SIO.gpio_oe(0).value_set().write_value(1 << self.dio_pin);
            if (pin_output & 0x02) != 0 {
                pac::SIO.gpio_out(0).value_set().write_value(1 << self.dio_pin);
            } else {
                pac::SIO.gpio_out(0).value_clr().write_value(1 << self.dio_pin);
            }
        }

        if (pin_select & 0x80) != 0 {
            self.assert_reset((pin_output & 0x80) == 0);
        }

        if wait_us > 0 {
            delay_cycles(wait_us * 150);
        }

        let in_val = pac::SIO.gpio_in(0).read();
        let mut status = 0u8;
        if (in_val & (1 << self.clk_pin)) != 0 {
            status |= 0x01;
        }
        if (in_val & (1 << self.dio_in_pin)) != 0 {
            status |= 0x02;
        }
        if (in_val & (1 << self.reset_pin)) != 0 {
            status |= 0x80;
        }

        if (pin_select & 0x03) != 0 {
            self.restore_pio_pins();
        }

        Ok(status)
    }
}

impl Adiv5Access for SwdTransport {
    fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        SwdTransport::write_dp(self, reg, value)
    }

    fn read_dp(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        SwdTransport::read_dp(self, reg, value)
    }

    fn select_ap(&mut self, apsel: u8, apbank: u8) -> Result<(), SwdError> {
        SwdTransport::select_ap(self, apsel, apbank)
    }

    fn power_up_debug(&mut self) -> Result<(), SwdError> {
        SwdTransport::power_up_debug(self)
    }

    fn recover_from_ack_error(&mut self) -> Result<(), SwdError> {
        SwdTransport::recover_from_ack_error(self)
    }

    fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        SwdTransport::write_ap(self, reg, value)
    }

    fn write_ap_port(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        SwdTransport::write_ap_port(self, apsel, reg, value)
    }

    fn read_ap(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        SwdTransport::read_ap(self, reg, value)
    }

    fn read_ap_port(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        SwdTransport::read_ap_port(self, apsel, reg, value)
    }

    fn raw_write_ap_no_recovery(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        SwdTransport::raw_write_ap_no_recovery(self, apsel, reg, value)
    }

    fn write_targetsel(&mut self, targetid: u32) {
        SwdTransport::write_targetsel(self, targetid)
    }

    fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError> {
        SwdTransport::read_dpidr_retry(self, idcode)
    }

    fn dormant_to_swd(&mut self) -> Result<(), SwdError> {
        SwdTransport::dormant_to_swd(self);
        self.ap_select = 0xFFFFFFFF;
        Ok(())
    }

    fn assert_reset(&mut self, assert: bool) -> Result<(), SwdError> {
        SwdIo::assert_reset(self, assert)
    }

    fn mem_read_chunk(&mut self, apsel: u8, addr: u32, words: &mut [u32]) -> Result<(), SwdError> {
        SwdTransport::mem_read_chunk(self, apsel, addr, words)
    }

    fn mem_write_chunk(&mut self, apsel: u8, addr: u32, words: &[u32]) -> Result<(), SwdError> {
        SwdTransport::mem_write_chunk(self, apsel, addr, words)
    }
}
