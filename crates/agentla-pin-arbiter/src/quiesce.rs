//! Hardware release notification hooks for pin deallocation.

/// Trait implemented by platform-specific hardware quiesce handlers.
///
/// When a pin claim handle is released via [`PinArbiter::release`](crate::PinArbiter::release)
/// or transient claims are cleared via [`PinArbiter::clear_transient`](crate::PinArbiter::clear_transient),
/// the arbiter invokes [`quiesce_pin`](PinQuiesce::quiesce_pin) for each pin returning to the free pool.
pub trait PinQuiesce {
    /// Return the designated physical pin to its safe / unconfigured state
    /// (typically SIO input, high impedance / output disabled, pulls disabled).
    fn quiesce_pin(&mut self, pin: u32);
}

impl PinQuiesce for () {
    #[inline]
    fn quiesce_pin(&mut self, _pin: u32) {}
}

impl<F: FnMut(u32)> PinQuiesce for F {
    #[inline]
    fn quiesce_pin(&mut self, pin: u32) {
        self(pin);
    }
}
