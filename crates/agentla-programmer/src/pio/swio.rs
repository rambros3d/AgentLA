//! RP2350 PIO hardware driver for WCH 1-wire SWIO.

use super::programs::{SWIO_INSTRUCTIONS, SWIO_WRAP_SOURCE, SWIO_WRAP_TARGET};
use crate::delay::delay_cycles;
use crate::swio::{SwioError, DM_DMCONTROL, WCH_DM_CFGR, WCH_DM_SHDWCFGR};
use crate::traits::SwioIo;
use rp_pac as pac;

pub struct SwioTransport {
    prog_offset: u8,
    pub swio_pin: usize,
    initialized: bool,
}

impl Default for SwioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl SwioTransport {
    pub const fn new() -> Self {
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
        pac::PADS_BANK0.gpio(swio_pin).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(false);
            w.set_pue(true);
            w.set_pde(false);
            w.set_slewfast(true);
            w.set_drive(pac::pads::vals::Drive::_4M_A);
        });

        self.prog_offset = 0;
        for (i, &ins) in SWIO_INSTRUCTIONS.iter().enumerate() {
            pac::PIO0.instr_mem(self.prog_offset as usize + i).write(|w| {
                w.set_instr_mem(ins);
            });
        }

        let sm = pac::PIO0.sm(0);

        sm.pinctrl().write(|w| {
            w.set_sideset_base(swio_pin as u8);
            w.set_sideset_count(1);
            w.set_out_base(swio_pin as u8);
            w.set_out_count(1);
            w.set_set_base(swio_pin as u8);
            w.set_set_count(1);
            w.set_in_base(swio_pin as u8);
        });

        sm.execctrl().write(|w| {
            w.set_wrap_bottom(self.prog_offset + SWIO_WRAP_TARGET);
            w.set_wrap_top(self.prog_offset + SWIO_WRAP_SOURCE);
            w.set_side_en(false);
            w.set_side_pindir(false);
        });

        sm.shiftctrl().write(|w| {
            w.set_out_shiftdir(false);
            w.set_in_shiftdir(false);
            w.set_autopull(false);
            w.set_autopush(true);
            w.set_push_thresh(32);
        });

        sm.clkdiv().write(|w| {
            w.set_int(14);
            w.set_frac(102);
        });

        sm.instr().write(|w| {
            w.set_instr(0x0000 | (self.prog_offset as u16));
        });

        pac::PIO0.ctrl().modify(|w| {
            w.set_sm_enable(1 << 0);
        });

        pac::IO_BANK0.gpio(swio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::SIOB_PROC_0.into());
        });
        pac::SIO.gpio_out(0).value_clr().write_value(1 << swio_pin);
        pac::SIO.gpio_oe(0).value_set().write_value(1 << swio_pin);
        delay_cycles(1500); // ~10 µs low reset pulse
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << swio_pin);
        pac::IO_BANK0.gpio(swio_pin).ctrl().write(|w| {
            w.set_funcsel(pac::io::vals::Gpio0ctrlFuncsel::PIO0_0.into());
        });

        delay_cycles(1500);

        let _ = self.put(WCH_DM_SHDWCFGR, 0x5AA50400);
        let _ = self.put(WCH_DM_CFGR, 0x5AA50400);
        let _ = self.put(DM_DMCONTROL, 0x00000000);
        let _ = self.put(DM_DMCONTROL, 0x80000001);

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
        for _ in 0..500_000 {
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
        while (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
            let _ = pac::PIO0.rxf(0).read();
        }

        let cmd = ((!addr) << 1) | 1;
        if !self.put_fifo(cmd) {
            return Err(SwioError::Timeout);
        }

        for _ in 0..500_000 {
            if (pac::PIO0.fstat().read().rxempty() & (1 << 0)) == 0 {
                return Ok(pac::PIO0.rxf(0).read());
            }
        }
        Err(SwioError::Timeout)
    }
}

impl SwioIo for SwioTransport {
    type Error = SwioError;

    fn put(&mut self, addr: u32, data: u32) -> Result<(), Self::Error> {
        SwioTransport::put(self, addr, data);
        Ok(())
    }

    fn get(&mut self, addr: u32) -> Result<u32, Self::Error> {
        SwioTransport::get(self, addr)
    }

    fn reset_sequence(&mut self) -> Result<(), Self::Error> {
        self.init_pin(self.swio_pin);
        Ok(())
    }
}
