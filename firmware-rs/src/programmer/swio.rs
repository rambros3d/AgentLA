use super::pio_programs::swio_program;
use rp_pac as pac;

pub const WCH_DM_CPBR: u32 = 0x7C;
pub const WCH_DM_CFGR: u32 = 0x7D;
pub const WCH_DM_SHDWCFGR: u32 = 0x7E;
pub const WCH_DM_PART: u32 = 0x7F;

pub const DM_DMCONTROL: u32 = 0x10;
pub const DM_DMSTATUS: u32 = 0x11;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwioError {
    Timeout,
    NotInit,
}

pub struct SwioTransport {
    prog_offset: u8,
    swio_pin: usize,
    initialized: bool,
}

impl SwioTransport {
    pub fn new() -> Self {
        Self {
            prog_offset: 0,
            swio_pin: 12,
            initialized: false,
        }
    }

    pub fn init(&mut self) {
        self.init_pin(12);
    }

    pub fn init_pin(&mut self, swio_pin: usize) {
        self.swio_pin = swio_pin;
        // 1. Configure GPIO pad
        pac::PADS_BANK0.gpio(swio_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true); // Pull-up is required for 1-wire open-drain bus
            w.set_pde(false);
            w.set_slewfast(true);
            w.set_drive(pac::pads::vals::Drive::_12M_A);
        });

        // 2. Load PIO program into PIO0 instruction memory
        let prog = swio_program();
        self.prog_offset = 0;
        for (i, &ins) in prog.code.iter().enumerate() {
            pac::PIO0.instr_mem(self.prog_offset as usize + i).write(|w| {
                w.set_instr_mem(ins);
            });
        }

        // 3. Configure State Machine 0
        let sm = pac::PIO0.sm(0);

        // All mapping on SWIO pin
        sm.pinctrl().write(|w| {
            w.set_sideset_base(swio_pin as u8);
            w.set_sideset_count(1);
            w.set_out_base(swio_pin as u8);
            w.set_out_count(1);
            w.set_set_base(swio_pin as u8);
            w.set_set_count(1);
            w.set_in_base(swio_pin as u8);
        });

        // Execctrl: sideset controls pindirs!
        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + prog.wrap.target);
            w.set_wrap_top(self.prog_offset + prog.wrap.source);
            w.set_side_en(false);
            w.set_side_pindir(true); // side_set 1 pindirs
        });

        // Shiftctrl: MSB first (false = LEFT), autopush 32 bits
        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(false);
            w.set_in_shiftdir(false);
            w.set_autopull(false);
            w.set_autopush(true);
            w.set_push_thresh(32);
        });

        // Clock divider: 150 MHz / 14.4 ≈ 10.4 MHz (approx 96ns tick)
        // 14 integer, frac = 0.4 * 256 ≈ 102
        sm.clkdiv().write(|w| {
            w.set_int(14);
            w.set_frac(102);
        });

        // Set SM pin output value to 0 for open-drain operation (side_pindir 1 drives LOW, 0 floats)
        sm.instr().write(|w| {
            w.set_instr(0b111_00000_000_00000); // set pins, 0
        });

        // Initial pin directions: input (0)
        sm.instr().write(|w| {
            w.set_instr(0b111_00000_100_00000); // set pindirs, 0
        });

        // Jump to wrap target (entry point 0)
        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset as u16));
        });

        // Enable SM0
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(1 << 0);
        });

        // 4. Send ~12µs low pulse on swio_pin to reset/awaken WCH debug module
        // Temporarily take over pin with SIO
        pac::IO_BANK0.gpio(swio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::SIO.gpio_out(0).value_clr().write_value(1 << swio_pin);
        pac::SIO.gpio_oe(0).value_set().write_value(1 << swio_pin);
        cortex_m::asm::delay(2000); // ~13 µs at 150 MHz
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << swio_pin);
        pac::IO_BANK0.gpio(swio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        cortex_m::asm::delay(5000); // ~33 µs settling high

        // 5. Awaken WCH target debug module
        self.put(WCH_DM_SHDWCFGR, 0x5AA50400);
        self.put(WCH_DM_CFGR, 0x5AA50400);
        self.put(DM_DMCONTROL, 0x00000000);
        self.put(DM_DMCONTROL, 0x00000001);

        self.initialized = true;
    }

    pub fn deinit(&mut self) {
        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(w.sm_enable() & !(1 << 0));
        });

        pac::PADS_BANK0.gpio(self.swio_pin).modify(|w| {
            w.set_pue(false);
            w.set_pde(false);
            w.set_od(true);
            w.set_ie(false);
        });
        pac::IO_BANK0.gpio(self.swio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::NULL.into());
        });

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

    pub fn put(&self, addr: u32, data: u32) {
        let cmd = ((!addr) << 1) | 0;
        self.put_fifo(cmd);
        self.put_fifo(!data);
    }

    pub fn get(&self, addr: u32) -> Result<u32, SwioError> {
        let cmd = ((!addr) << 1) | 1;
        if !self.put_fifo(cmd) {
            return Err(SwioError::Timeout);
        }

        for _ in 0..20_000 {
            if (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
                return Ok(pac::PIO0.rxf(0).read());
            }
        }
        Err(SwioError::Timeout)
    }
}
