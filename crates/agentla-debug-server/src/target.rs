//! Target access abstraction traits for memory inspection and debugging.

use crate::gdb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BreakpointError {
    NoFreeSlot,
    TargetError,
}

/// Abstract memory access trait for target RAM / flash inspection.
pub trait MemoryAccess {
    type Error: core::fmt::Debug;

    /// Read 32-bit word from aligned memory address.
    fn mem_read32(&mut self, addr: u32) -> Result<u32, Self::Error>;

    /// Write 32-bit word to aligned memory address.
    fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error>;

    /// Read an arbitrary block of bytes from memory.
    fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Write an arbitrary block of bytes to memory.
    fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error>;
}

/// Abstract debug target trait for GDB RSP stubs.
pub trait GdbTarget: MemoryAccess {
    /// Check whether target core is halted.
    fn is_halted(&mut self) -> Result<bool, Self::Error>;

    /// Halt target core.
    fn halt(&mut self) -> Result<(), Self::Error>;

    /// Resume target core execution.
    fn resume(&mut self) -> Result<(), Self::Error>;

    /// Single-step one instruction.
    fn step(&mut self) -> Result<(), Self::Error>;

    /// Read core register (R0-R15 + xPSR).
    fn read_core_reg(&mut self, reg: u8) -> Result<u32, Self::Error>;

    /// Write core register (R0-R15 + xPSR).
    fn write_core_reg(&mut self, reg: u8, val: u32) -> Result<(), Self::Error>;

    /// Insert hardware breakpoint. Default implementation uses CoreSight FPB.
    fn insert_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        gdb::insert_hw_breakpoint(self, addr)
    }

    /// Remove hardware breakpoint. Default implementation uses CoreSight FPB.
    fn remove_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        gdb::remove_hw_breakpoint(self, addr)
    }
}

impl<T: MemoryAccess + ?Sized> MemoryAccess for &mut T {
    type Error = T::Error;
    #[inline]
    fn mem_read32(&mut self, addr: u32) -> Result<u32, Self::Error> {
        (**self).mem_read32(addr)
    }
    #[inline]
    fn mem_write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error> {
        (**self).mem_write32(addr, val)
    }
    #[inline]
    fn mem_read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error> {
        (**self).mem_read_block(addr, buf)
    }
    #[inline]
    fn mem_write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error> {
        (**self).mem_write_block(addr, data)
    }
}

impl<T: GdbTarget + ?Sized> GdbTarget for &mut T {
    #[inline]
    fn is_halted(&mut self) -> Result<bool, Self::Error> {
        (**self).is_halted()
    }
    #[inline]
    fn halt(&mut self) -> Result<(), Self::Error> {
        (**self).halt()
    }
    #[inline]
    fn resume(&mut self) -> Result<(), Self::Error> {
        (**self).resume()
    }
    #[inline]
    fn step(&mut self) -> Result<(), Self::Error> {
        (**self).step()
    }
    #[inline]
    fn read_core_reg(&mut self, reg: u8) -> Result<u32, Self::Error> {
        (**self).read_core_reg(reg)
    }
    #[inline]
    fn write_core_reg(&mut self, reg: u8, val: u32) -> Result<(), Self::Error> {
        (**self).write_core_reg(reg, val)
    }
    #[inline]
    fn insert_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        (**self).insert_breakpoint(addr)
    }
    #[inline]
    fn remove_breakpoint(&mut self, addr: u32) -> Result<(), BreakpointError> {
        (**self).remove_breakpoint(addr)
    }
}
