//! Abstract target bus and memory access traits.

/// Hardware-independent memory and bus access operations for target controllers.
pub trait TargetMemAccess {
    type Error: core::fmt::Debug;

    /// Read a single 32-bit word from the target memory address space.
    fn read32(&mut self, addr: u32) -> Result<u32, Self::Error>;

    /// Write a single 32-bit word to the target memory address space.
    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error>;

    /// Read a contiguous block of bytes from the target address space.
    fn read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error>;

    /// Write a contiguous block of bytes to the target address space.
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error>;

    /// Optional target hardware reset assertion / deassertion.
    fn assert_reset(&mut self, _assert: bool) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Optional target core halt.
    fn halt(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Optional target core resume.
    fn resume(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}
