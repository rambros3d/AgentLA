# domain-crate-bridges Specification

## Purpose
TBD - created by archiving change modularize-bridges-crate. Update Purpose after archive.
## Requirements
### Requirement: Standalone Bridges Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-bridges` configured as a `#![no_std]` library with optional `defmt` and `std` features.

#### Scenario: Compiling on host targets
- **WHEN** building `agentla-bridges` on an x86_64 host with `cargo test --package agentla-bridges`
- **THEN** the crate compiles successfully and runs all unit tests without requiring MCU HAL or PAC dependencies.

### Requirement: Linux i2c-tiny-usb Protocol Engine
The `agentla-bridges` crate SHALL provide a protocol handler conforming to the Linux kernel `i2c-tiny-usb` specification operating over standard `embedded-hal::i2c::I2c`.

#### Scenario: Handling CMD_I2C_IO read transfer
- **WHEN** a `CMD_I2C_IO` request with `CMD_I2C_IO_BEGIN | CMD_I2C_IO_END` and read flag is dispatched
- **THEN** the handler performs an I2C read operation on the underlying I2C bus and returns `STATUS_ADDRESS_ACK` with data bytes.

#### Scenario: Handling CMD_GET_FUNC
- **WHEN** a `CMD_GET_FUNC` request is received
- **THEN** the handler returns the supported I2C and SMBus functionality flags (`I2C_FUNC_ALL`).

### Requirement: Linux SocketCAN gs_usb Protocol Engine
The `agentla-bridges` crate SHALL provide request handling, bit timing validation, and frame serialization conforming to the Linux kernel `gs_usb` specification.

#### Scenario: Handling GS_USB_BREQ_HOST_FORMAT
- **WHEN** host sends a format negotiation request
- **THEN** the handler validates the byte order and returns `GS_CAN_FEATURE_LISTEN_ONLY | GS_CAN_FEATURE_LOOP_BACK | GS_CAN_FEATURE_TRIPLE_SAMPLE`.

#### Scenario: Packing and unpacking CAN frames
- **WHEN** converting between native CAN frames and `gs_host_frame` structures
- **THEN** echo ID, CAN identifier (standard and extended), data length code (DLC), and payload bytes are preserved with correct endianness.

### Requirement: Lawicel SLCAN ASCII Protocol Engine
The `agentla-bridges` crate SHALL provide bidirectional parsing and formatting between ASCII commands and CAN frames.

#### Scenario: Parsing standard transmit command
- **WHEN** receiving ASCII string `t123411223344\r`
- **THEN** the parser returns a standard CAN frame with ID 0x123, DLC 4, and payload `[0x11, 0x22, 0x33, 0x44]`.

#### Scenario: Formatting received frame to ASCII
- **WHEN** serializing a received extended CAN frame with ID 0x18DAF110, DLC 2, and payload `[0x02, 0x01]`
- **THEN** the formatter generates `T18DAF11020201\r`.

