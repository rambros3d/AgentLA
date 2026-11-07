## ADDED Requirements

### Requirement: Standalone Analyzer Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-analyzer` configured as a `#![no_std]` library with optional `std` support for host compilation and testing.

#### Scenario: Compiling on host targets
- **WHEN** building `agentla-analyzer` on an x86_64 host with `cargo test --package agentla-analyzer`
- **THEN** the crate compiles successfully and runs all unit tests without requiring MCU HAL or PAC dependencies.

#### Scenario: Compiling on embedded targets
- **WHEN** building `agentla-analyzer` with `default-features = false` for target `thumbv8m.main-none-eabihf`
- **THEN** the crate compiles in pure `#![no_std]` mode.

### Requirement: Bit-Delta Run-Length Compression Engine
The `agentla-analyzer` crate SHALL provide bit-delta transformation, untransformation, block compression, and decompression algorithms operating on word slices without heap allocation.

#### Scenario: Round-trip compression and decompression
- **WHEN** a block of 32-bit sample words is passed to `bit_delta_compress_block`
- **THEN** the resulting compressed byte stream decompresses via `bit_delta_decompress_block` to match the exact original sample words.

### Requirement: Value Change Dump Generator
The `agentla-analyzer` crate SHALL provide a VCD (Value Change Dump) generator capable of formatting captured digital transitions into valid VCD text streams.

#### Scenario: Generating standard VCD header and timestamps
- **WHEN** sample buffers and signal channel masks are passed to the VCD generator
- **THEN** the generator formats valid `$date`, `$version`, `$timescale`, and signal transition records into the output buffer.

### Requirement: Bus Protocol Decoders
The `agentla-analyzer` crate SHALL provide software decoders for I2C, SPI, UART, CAN, SWD, and 1-wire protocols that process raw digital sample buffers.

#### Scenario: Decoding SWD transaction
- **WHEN** digital samples containing an SWD clock and data transaction are provided to `swd_decode`
- **THEN** the decoder returns the parsed request, ACK status, and data payload.

#### Scenario: Decoding I2C transaction
- **WHEN** digital samples containing I2C SCL and SDA signals are provided to `i2c_decode_transactions`
- **THEN** the decoder returns start conditions, 7-bit addresses, read/write flags, ACK/NACK bits, and stop conditions.
