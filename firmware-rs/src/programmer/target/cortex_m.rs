//! Arm Cortex-M core control.

pub use agentla_programmer::target::cortex_m::*;
pub type CortexM<'a> = agentla_programmer::target::cortex_m::CortexM<'a, crate::programmer::swd::SwdTransport>;

use agentla_debug_server::target::{BreakpointError, GdbTarget, MemoryAccess};

pub struct CortexTarget<'a, 'b, S: agentla_programmer::swd::Adiv5Access>(
    pub &'a mut agentla_programmer::target::cortex_m::CortexM<'b, S>,
);

impl<'a, 'b, S: agentla_programmer::swd::Adiv5Access> MemoryAccess for CortexTarget<'a, 'b, S> {
    type Error = CortexError;

    #[inline]
    fn mem_read32(&mut self, addr: u32) -> Result<u32, Self::Error> {
        self.0.mem_read32(addr)
    }

    #[inline]
    fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error> {
        self.0.mem_write32(addr, val)
    }

    #[inline]
    fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.0.mem_read_block(addr, buf)
    }

    #[inline]
    fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error> {
        self.0.mem_write_block(addr, data)
    }
}

impl<'a, 'b, S: agentla_programmer::swd::Adiv5Access> GdbTarget for CortexTarget<'a, 'b, S> {
    #[inline]
    fn is_halted(&mut self) -> Result<bool, Self::Error> {
        self.0.is_halted()
    }

    #[inline]
    fn halt(&mut self) -> Result<(), Self::Error> {
        self.0.halt()
    }

    #[inline]
    fn resume(&mut self) -> Result<(), Self::Error> {
        self.0.resume()
    }

    fn step(&mut self) -> Result<(), Self::Error> {
        let _ = self.0.mem_write32(
            DHCSR,
            DHCSR_DBGKEY | DHCSR_C_DEBUGEN | DHCSR_C_STEP,
        );
        for _ in 0..1000 {
            if let Ok(dhcsr) = self.0.mem_read32(DHCSR) {
                if (dhcsr & DHCSR_S_HALT) != 0 {
                    break;
                }
            }
            cortex_m::asm::delay(1500);
        }
        Ok(())
    }

    #[inline]
    fn read_core_reg(&mut self, reg: u8) -> Result<u32, Self::Error> {
        self.0.read_core_reg(reg)
    }

    #[inline]
    fn write_core_reg(&mut self, reg: u8, val: u32) -> Result<(), Self::Error> {
        self.0.write_core_reg(reg, val)
    }

    #[inline]
    fn insert_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        agentla_debug_server::gdb::insert_hw_breakpoint(self, addr)
    }

    #[inline]
    fn remove_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        agentla_debug_server::gdb::remove_hw_breakpoint(self, addr)
    }
}
