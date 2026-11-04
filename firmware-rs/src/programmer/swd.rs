use super::pio_programs::{
    swd_program, SWD_OFFSET_GET_NEXT_CMD, SWD_OFFSET_READ_CMD, SWD_OFFSET_TURNAROUND_CMD,
    SWD_OFFSET_WRITE_CMD,
};
use rp_pac as pac;

pub const SWD_ACK_OK: u8 = 0x01;
pub const SWD_ACK_WAIT: u8 = 0x02;
pub const SWD_ACK_FAULT: u8 = 0x04;
pub const SWD_ACK_PARITY_ERR: u8 = 0x05;
pub const SWD_ACK_TIMEOUT: u8 = 0x07;

pub const DP_DPIDR: u8 = 0x00;
pub const DP_ABORT: u8 = 0x00;
pub const DP_CTRLSTAT: u8 = 0x04;
pub const DP_SELECT: u8 = 0x08;
pub const DP_RDBUFF: u8 = 0x0C;

pub const AP_CSW: u8 = 0x00;
pub const AP_TAR: u8 = 0x04;
pub const AP_DRW: u8 = 0x0C;
pub const AP_IDR: u8 = 0xFC;

pub const ADIV5_AP_CSW_DEFAULT: u32 = 0x23000052;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwdError {
    AckWait,
    AckFault,
    ParityError,
    ProtocolError,
    Timeout,
    NoTarget,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
enum ProbeCmd {
    Write,
    Skip,
    Turnaround,
    Read,
}

pub struct SwdTransport {
    prog_offset: u8,
    turnaround: u8,
    idle_cycles: u8,
    ap_select: u32,
    clock_khz: u32,
}

impl SwdTransport {
    pub fn new() -> Self {
        Self {
            prog_offset: 0,
            turnaround: 1,
            idle_cycles: 8,
            ap_select: 0xFFFFFFFF,
            clock_khz: 1000,
        }
    }

    pub fn init(&mut self, freq_khz: u32) {
        // 1. Configure GPIO pads
        // SWCLK: GPIO 4
        pac::PADS_BANK0.gpio(4).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
            w.set_slewfast(true);
            w.set_drive(pac::pads::vals::Drive::_12M_A);
        });
        pac::IO_BANK0.gpio(4).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        // SWDIO: GPIO 5 (with pull-up, idle is high)
        pac::PADS_BANK0.gpio(5).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
            w.set_slewfast(true);
            w.set_drive(pac::pads::vals::Drive::_12M_A);
        });
        pac::IO_BANK0.gpio(5).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        // nRESET: GPIO 6 (pull-up, open-drain emulation: input by default)
        pac::PADS_BANK0.gpio(6).modify(|w| {
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::IO_BANK0.gpio(6).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });

        // 2. Load PIO Program into PIO0 instruction memory
        let prog = swd_program();
        self.prog_offset = 0;
        for (i, &ins) in prog.code.iter().enumerate() {
            pac::PIO0.instr_mem(self.prog_offset as usize + i).write(|w| {
                w.set_instr_mem(ins);
            });
        }

        // 3. Configure State Machine 0
        let sm = pac::PIO0.sm(0);

        // Sideset: SWCLK = GPIO 4, 1 bit optional pin value
        sm.pinctrl().write(|w| {
            w.set_sideset_base(4);
            w.set_sideset_count(2); // 1 bit value + 1 bit optional enable
            w.set_out_base(5);
            w.set_out_count(1);
            w.set_set_base(5);
            w.set_set_count(1);
            w.set_in_base(5);
        });

        // Execctrl: wrap boundaries (10 down to 3) + side-set opt enable
        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + prog.wrap.target);
            w.set_wrap_top(self.prog_offset + prog.wrap.source);
            w.set_side_en(true);
            w.set_side_pindir(false);
        });

        // Shiftctrl: shift right (true), no autopull, no autopush
        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(true);
            w.set_in_shiftdir(true);
            w.set_autopull(false);
            w.set_autopush(false);
        });

        // Pin directions: SWCLK (4) & SWDIO (5) start as outputs
        sm.instr().write(|w| {
            // set pindirs, 3 on pins 4..5
            w.set_instr(0b111_00000_100_00011);
        });

        // Set clock frequency
        self.set_clock(freq_khz);

        // Clear FIFOs
        sm.instr().write(|w| w.set_instr(0x80a0)); // pull
        pac::PIO0.fdebug().write(|w| w.0 = 0xFFFFFFFF);

        // Jump to command dispatcher
        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset + SWD_OFFSET_GET_NEXT_CMD) as u16);
        });

        // Enable SM0
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(1 << 0);
        });

        self.ap_select = 0xFFFFFFFF;
    }

    pub fn deinit(&mut self) {
        // Disable SM0
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(w.sm_enable() & !(1 << 0));
        });

        // Tristate pins
        for pin in [4, 5] {
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
    }

    pub fn set_clock(&mut self, freq_khz: u32) {
        let clk_sys_khz = 150_000u32;
        let mut div = (((clk_sys_khz + freq_khz - 1) / freq_khz) + 3) / 4;
        if div == 0 {
            div = 1;
        }
        if div > 65535 {
            div = 65535;
        }

        pac::PIO0.sm(0).clkdiv().write(|w| {
            w.set_int(div as u16);
            w.set_frac(0);
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
        for _ in 0..10_000 {
            if (pac::PIO0.fstat().read().txfull() & (1 << 0)) == 0 {
                pac::PIO0.txf(0).write_value(data);
                return true;
            }
        }
        false
    }

    pub fn write_bits(&self, bit_count: u32, data: u32) {
        if bit_count == 0 {
            return;
        }
        if !self.put_fifo(self.fmt_command(bit_count, true, ProbeCmd::Write)) {
            return;
        }
        self.put_fifo(data);
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
            return 0;
        }

        for _ in 0..20_000 {
            if (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
                let data = pac::PIO0.rxf(0).read();
                return if bit_count < 32 {
                    data >> (32 - bit_count)
                } else {
                    data
                };
            }
        }
        0
    }

    pub fn line_reset(&mut self) {
        // 60 HIGH cycles + 8 LOW idle cycles
        self.write_bits(32, 0xFFFFFFFF);
        self.write_bits(28, 0x0FFFFFFF);
        self.write_bits(8, 0x00000000);
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn jtag_to_swd(&mut self) {
        // 1. Line reset (56 HIGH cycles)
        self.write_bits(32, 0xFFFFFFFF);
        self.write_bits(24, 0x00FFFFFF);

        // 2. JTAG-to-SWD switch code 0xE79E (16 bits LSB first)
        self.write_bits(16, 0xE79E);

        // 3. Post-switch line reset (60 HIGH cycles)
        self.write_bits(32, 0xFFFFFFFF);
        self.write_bits(28, 0x0FFFFFFF);

        // 4. Idle cycles (8 LOW bits)
        self.write_bits(8, 0x00);
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn dormant_to_swd(&mut self) {
        // ARM ADIv5.2 / ADIv6 Dormant-to-SWD Sequence (199 bits)
        self.write_bits(32, 0x09F392FF);
        self.write_bits(32, 0x852D9562);
        self.write_bits(32, 0xDDAFE986);
        self.write_bits(32, 0xBC0EA2E3);
        self.write_bits(32, 0xFFFA1019);
        self.write_bits(32, 0xFFFFFFFF);
        self.write_bits(7, 0x0000003F);
        self.ap_select = 0xFFFFFFFF;
    }

    pub fn build_request(is_ap: bool, is_read: bool, a23: u8) -> u8 {
        let mut prq = 1u8; // Start bit
        let ap = if is_ap { 1u8 } else { 0u8 };
        let rnw = if is_read { 1u8 } else { 0u8 };
        let a2 = a23 & 1;
        let a3 = (a23 >> 1) & 1;

        prq |= ap << 1;
        prq |= rnw << 2;
        prq |= a2 << 3;
        prq |= a3 << 4;

        let parity = (ap + rnw + a2 + a3) & 1;
        prq |= parity << 5;
        // Stop bit = 0 (bit 6)
        prq |= 1 << 7; // Park bit = 1
        prq
    }

    pub fn transfer(&self, request: u8, data: &mut u32) -> u8 {
        // 1. Send request byte
        self.write_bits(8, request as u32);

        // 2. Turnaround + read ACK (3 bits)
        let raw_ack = self.read_bits((self.turnaround + 3) as u32);
        let ack = (raw_ack >> self.turnaround) as u8 & 0x07;

        let is_read = (request & (1 << 2)) != 0;

        if ack == SWD_ACK_OK {
            if is_read {
                // Read 32 bits data + 1 bit parity
                let val = self.read_bits(32);
                let bit = self.read_bits(1) as u8;
                let parity = (val.count_ones() & 1) as u8;
                if parity != bit {
                    self.hiz_clocks(self.turnaround as u32);
                    return SWD_ACK_PARITY_ERR;
                }
                *data = val;
                self.hiz_clocks(self.turnaround as u32);
            } else {
                // Write turnaround
                self.hiz_clocks(self.turnaround as u32);
                let val = *data;
                self.write_bits(32, val);
                let parity = val.count_ones() & 1;
                self.write_bits(1, parity);
            }

            if self.idle_cycles > 0 {
                self.write_bits(self.idle_cycles as u32, 0);
            }
            return SWD_ACK_OK;
        }

        if ack == SWD_ACK_WAIT || ack == SWD_ACK_FAULT {
            self.hiz_clocks(self.turnaround as u32);
            return ack;
        }

        // Protocol error: back off
        self.read_bits((self.turnaround as u32) + 33);
        ack
    }

    pub fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        let req = Self::build_request(false, false, (reg >> 2) & 3);
        let mut val = value;
        for _ in 0..50 {
            let ack = self.transfer(req, &mut val);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                continue;
            } else if ack == SWD_ACK_FAULT {
                return Err(SwdError::AckFault);
            } else {
                return Err(SwdError::ProtocolError);
            }
        }
        Err(SwdError::Timeout)
    }

    pub fn read_dp(&self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let req = Self::build_request(false, true, (reg >> 2) & 3);
        for _ in 0..50 {
            let ack = self.transfer(req, value);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                continue;
            } else if ack == SWD_ACK_FAULT {
                return Err(SwdError::AckFault);
            } else if ack == SWD_ACK_PARITY_ERR {
                return Err(SwdError::ParityError);
            } else {
                return Err(SwdError::ProtocolError);
            }
        }
        Err(SwdError::Timeout)
    }

    pub fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        let req = Self::build_request(true, false, (reg >> 2) & 3);
        let mut val = value;
        for _ in 0..50 {
            let ack = self.transfer(req, &mut val);
            if ack == SWD_ACK_OK {
                return Ok(());
            } else if ack == SWD_ACK_WAIT {
                continue;
            } else if ack == SWD_ACK_FAULT {
                return Err(SwdError::AckFault);
            } else {
                return Err(SwdError::ProtocolError);
            }
        }
        Err(SwdError::Timeout)
    }

    pub fn read_ap(&self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let req = Self::build_request(true, true, (reg >> 2) & 3);
        let mut dummy = 0u32;
        // AP reads are pipelined: initial read triggers transfer
        let ack = self.transfer(req, &mut dummy);
        if ack != SWD_ACK_OK {
            return Err(SwdError::ProtocolError);
        }
        // Read RDBUFF to get actual data
        self.read_dp(DP_RDBUFF, value)
    }

    pub fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError> {
        let mut res = self.read_dp(DP_DPIDR, idcode);
        let freqs = [100, 200, 500, 1000];
        let mut freq_idx = 0;

        for retry in 0..30 {
            if res.is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }
            if retry > 0 && retry % 5 == 0 && freq_idx < 3 {
                freq_idx += 1;
                self.set_clock(freqs[freq_idx]);
            }
            // Clear sticky errors via ABORT register
            let _ = self.write_dp(DP_ABORT, 0x0000001E);
            // Power up request CDBGPWRUPREQ | CSYSPWRUPREQ
            let _ = self.write_dp(DP_CTRLSTAT, 0x50000000);
            res = self.read_dp(DP_DPIDR, idcode);
        }
        res
    }
}
