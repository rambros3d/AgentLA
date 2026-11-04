use super::pio_programs::{
    rvswd_program, RVSWD_OFFSET_GET_NEXT_CMD, RVSWD_OFFSET_READ_BITS, RVSWD_OFFSET_RESET,
    RVSWD_OFFSET_START, RVSWD_OFFSET_STOP, RVSWD_OFFSET_WRITE_BITS,
};
use rp_pac as pac;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RvswdError {
    Timeout,
    ParityError,
    NotInit,
}

pub struct RvswdTransport {
    prog_offset: u8,
    clkdiv: u16,
    initialized: bool,
}

impl RvswdTransport {
    pub fn new() -> Self {
        Self {
            prog_offset: 0,
            clkdiv: 36, // 150 MHz / 36 ≈ 4.17 MHz
            initialized: false,
        }
    }

    pub fn init(&mut self) {
        // 1. Configure GPIO pads
        // SWCLK: GPIO 4
        pac::PADS_BANK0.gpio(4).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
            w.set_slewfast(false);
            w.set_drive(pac::pads::vals::Drive::_2M_A);
        });
        pac::IO_BANK0.gpio(4).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        // SWDIO: GPIO 5 (pull-up enabled)
        pac::PADS_BANK0.gpio(5).modify(|w| {
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
            w.set_slewfast(false);
            w.set_drive(pac::pads::vals::Drive::_2M_A);
        });
        pac::IO_BANK0.gpio(5).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        // 2. Load PIO program into PIO0
        let prog = rvswd_program();
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

        // Execctrl: wrap boundaries + side-set opt enable
        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + prog.wrap.target);
            w.set_wrap_top(self.prog_offset + prog.wrap.source);
            w.set_side_en(true);
            w.set_side_pindir(false);
        });

        // Shiftctrl: MSB first (LEFT = false), no autopull, no autopush
        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(false);
            w.set_in_shiftdir(false);
            w.set_autopull(false);
            w.set_autopush(false);
        });

        // Clock divider: 36 integer (~4.17 MHz)
        sm.clkdiv().write(|w| {
            w.set_int(self.clkdiv);
            w.set_frac(0);
        });

        // Clear FIFOs and reset debug flags
        pac::PIO0.fdebug().write(|w| w.0 = 0xFFFFFFFF);

        // Jump to command dispatcher
        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset + RVSWD_OFFSET_GET_NEXT_CMD) as u16);
        });

        // Enable SM0
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
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
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
        cortex_m::asm::delay(75_000); // ~500 µs

        // Awaken WCH Debug Module: set dmactive
        self.write(0x10, 0x80000001)?;
        self.write(0x10, 0x80000001)?;
        self.write(0x10, 0x00000001)?;
        cortex_m::asm::delay(750_000); // ~5 ms

        Ok(())
    }

    fn build_header(reg: u8, is_write: bool) -> u32 {
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

    pub fn write(&self, reg: u8, value: u32) -> Result<(), RvswdError> {
        if !self.start_seq() {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(Self::build_header(reg, true), 9) {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(0b10101, 5) {
            // Turnaround
            return Err(RvswdError::Timeout);
        }

        let mut parity = false;
        for i in 0..32 {
            if ((value >> (31 - i)) & 1) != 0 {
                parity = !parity;
            }
        }
        if !self.write_bits(value, 32) {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(if parity { 1 } else { 0 }, 1) {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(0b10111, 5) {
            // Trailer
            return Err(RvswdError::Timeout);
        }
        if !self.stop_seq() {
            return Err(RvswdError::Timeout);
        }
        Ok(())
    }

    pub fn read(&self, reg: u8) -> Result<u32, RvswdError> {
        if !self.start_seq() {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(Self::build_header(reg, false), 9) {
            return Err(RvswdError::Timeout);
        }
        if !self.write_bits(0b10101, 5) {
            // Turnaround
            return Err(RvswdError::Timeout);
        }

        let data = self.read_bits(32)?;
        let parity_read = self.read_bits(1)?;

        if !self.write_bits(0b10111, 5) {
            // Trailer
            return Err(RvswdError::Timeout);
        }
        if !self.stop_seq() {
            return Err(RvswdError::Timeout);
        }

        let mut data_parity = false;
        for i in 0..32 {
            if ((data >> (31 - i)) & 1) != 0 {
                data_parity = !data_parity;
            }
        }

        let parity_bit = (parity_read >> 31) & 1 == 1;
        if data_parity != parity_bit {
            return Err(RvswdError::ParityError);
        }
        Ok(data)
    }
}
