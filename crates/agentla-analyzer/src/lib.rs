#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod compression;
pub mod config;
pub mod decoders;
pub mod trigger;
pub mod vcd;

pub use compression::{
    bit_delta_compress_block, bit_delta_decompress_block, bit_delta_transform_u32,
    bit_delta_untransform_u32, rle_decode, rle_encode, TAG_WORD_DELTA, TAG_ZERO_SHORT,
};
pub use config::{
    capture_get_engine_mode_name, program_names, CaptureConfig, Error, Mode, State, Status,
    TriggerConfig, TriggerType, BANK_WORDS, SAMPLE_COUNT_MAX,
};
pub use trigger::{falling_edge, rising_edge};
pub use vcd::{generate_vcd, generate_vcd_custom};
