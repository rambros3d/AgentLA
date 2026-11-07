## ADDED Requirements

### Requirement: Standalone Programmer Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-programmer` configured as a `#![no_std]` library with an optional `rp2350-pio` hardware feature.

#### Scenario: Compiling on host targets without hardware dependencies
- **WHEN** building `agentla-programmer` on an x86_64 host with `cargo test --package agentla-programmer`
- **THEN** the crate compiles successfully using mock transport implementations without linking `rp-pac` or Cortex-M runtime crates.

#### Scenario: Compiling with RP2350 PIO support
- **WHEN** building `agentla-programmer` with feature `rp2350-pio` enabled for target `thumbv8m.main-none-eabihf`
- **THEN** the crate compiles with PIO instruction programs and PIO FIFO drivers.

### Requirement: Abstract Hardware Transport Traits
The `agentla-programmer` crate SHALL define abstract traits `SwdIo` and `SwioIo` decoupling high-level debug protocol sequences from underlying physical pin toggling and PIO state machines.

#### Scenario: Executing protocol sequences over abstract trait
- **WHEN** a debug operation invokes `transfer` or `reset_sequence` on an instance implementing `SwdIo`
- **THEN** the protocol logic successfully reads and writes registers regardless of whether the backend is physical PIO hardware or a mock test driver.

### Requirement: CMSIS-DAP Packet Handling
The `agentla-programmer` crate SHALL provide CMSIS-DAP v1 and v2 command parsing, execution, and response serialization independent of USB transport layers.

#### Scenario: Handling DAP_Transfer command
- **WHEN** a valid DAP_Transfer command packet is received by `CmsisDapHandler`
- **THEN** the handler executes the transfers via the underlying `SwdIo` transport and serializes a compliant response buffer with status and data words.

### Requirement: Arm ADIv5 and WCH QingKe Protocol Engines
The `agentla-programmer` crate SHALL provide state machines for Arm ADIv5 SWD (DP/AP accesses, dormant-to-SWD sequences) and WCH QingKe debug module protocols (1-wire SWIO and 2-wire RVSWD).

#### Scenario: Target identification via SWD
- **WHEN** connecting to an Arm target via `SwdTransport`
- **THEN** the protocol engine reads DPIDR, initializes CTRL/STAT, and identifies the target core family.

#### Scenario: Target identification via WCH SWIO
- **WHEN** connecting to a CH32V target via `SwioTransport`
- **THEN** the protocol engine initializes DMSTATUS, reads part ID, and unlocks the debug interface.

### Requirement: Flash Algorithm and Diagnostic Engine
The `agentla-programmer` crate SHALL provide target memory access, flash algorithm execution (CMSIS FLM loader), and vendor-specific rescue routines (e.g., NRF unprotect).

#### Scenario: Executing RAM flash algorithm
- **WHEN** flashing target flash memory with an FLM blob
- **THEN** the algorithm runner loads the algorithm code into target RAM, sets stack and program counter, and verifies successful page write and erase.
