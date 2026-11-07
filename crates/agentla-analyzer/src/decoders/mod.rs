//! Protocol decoders module.

pub mod can;
pub mod i2c;
pub mod i3c;
pub mod led;
pub mod spi;
pub mod swd;

pub use can::{can_decode, CanMsg, CanResult};
pub use i2c::{i2c_decode_transactions, I2cDecodedTransaction};
pub use i3c::{i3c_decode, I3cResult};
pub use led::{decoder_count, led_decode_1w, led_decode_2w, LedResult};
pub use spi::{spi_decode_transactions, SpiDecodedTransaction};
pub use swd::{swd_decode, SwdAck, SwdReq, SwdResult, SWD_KAT, SWD_TEST_SAMPLE};
