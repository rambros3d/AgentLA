//! Protocol decoders module.

pub mod can;
pub mod event;
pub mod i2c;
pub mod i3c;
pub mod led;
pub mod matrix;
pub mod spi;
pub mod swd;
pub mod uart;

pub use can::{can_decode, CanMsg, CanResult};
pub use event::{DecodedEvent, ProtocolKind, Transition};
pub use i2c::{i2c_decode_transactions, I2cDecodedTransaction};
pub use i3c::{i3c_decode, I3cResult};
pub use led::{decoder_count, led_decode_1w, led_decode_2w, LedResult};
pub use matrix::{HardwareTriggerMatrix, HardwareTriggerRule, TriggerActionOutput, MAX_RULES};
pub use spi::{spi_decode_transactions, SpiDecodedTransaction};
pub use swd::{swd_decode, SwdAck, SwdReq, SwdResult, SWD_KAT, SWD_TEST_SAMPLE};
pub use uart::{Parity, UartConfig, UartDecoder};

/// Standard interface for streaming protocol decoders.
pub trait ProtocolDecoder {
    fn reset(&mut self);
    fn process_transition(&mut self, timestamp_ticks: u32, pin_states: u32) -> Option<DecodedEvent>;
}
