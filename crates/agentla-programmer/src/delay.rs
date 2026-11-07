//! Architecture-independent timing delays.

#[inline(always)]
pub fn delay_cycles(cycles: u32) {
    #[cfg(target_arch = "arm")]
    {
        let count = cycles / 3;
        if count > 0 {
            unsafe {
                core::arch::asm!(
                    "1:",
                    "subs {0}, {0}, #1",
                    "bne 1b",
                    inout(reg) count => _,
                    options(nomem, nostack),
                );
            }
        }
    }
    #[cfg(not(target_arch = "arm"))]
    {
        core::hint::black_box(cycles);
    }
}
