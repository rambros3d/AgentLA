#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod compression;
pub mod config;
pub mod decoders;
pub use decoders as decoder;
pub mod ols;
pub mod stream;
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
pub use ols::{
    generate_ols_metadata, sump_rle_encode_8bit, OlsCommand, OlsParser, CMD_ID, CMD_METADATA,
    CMD_RESET, CMD_RUN, CMD_SET_DIVIDER, CMD_SET_FLAGS, CMD_SET_READ_COUNT, OLS_ID_RESPONSE,
};
pub use stream::{
    SpscStreamBuffer, StreamFrameHeader, FLAG_FORMAT_BIT_DELTA, FLAG_FORMAT_SUMP_RLE,
    FLAG_OVERFLOW, FRAME_MAGIC,
};
pub use trigger::{falling_edge, rising_edge};
pub use vcd::{generate_vcd, generate_vcd_custom};
