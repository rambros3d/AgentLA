## ADDED Requirements

### Requirement: Standalone Debug Server Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-debug-server` configured as a `#![no_std]` library with optional `defmt` and `std` features.

#### Scenario: Compiling and testing on host targets
- **WHEN** building `agentla-debug-server` on an x86_64 host with `cargo test --package agentla-debug-server`
- **THEN** all GDB RSP packet framing and RTT parsing unit tests compile and pass without network or MCU HAL dependencies.

### Requirement: GDB Remote Serial Protocol Parser
The `agentla-debug-server` crate SHALL provide an RSP packet parser and response generator supporting standard GDB queries (`qSupported`, `?`, `g`, `G`, `m`, `M`, `c`, `s`, `vCont`, `Z0`, `z0`).

#### Scenario: Processing qSupported query
- **WHEN** host sends `$qSupported:multiprocess+;swbreak+;hwbreak+#...`
- **THEN** the server returns `$PacketSize=1024;qXfer:memory-map:read-;hwbreak+#...`.

#### Scenario: Checksum verification and ACK handling
- **WHEN** an RSP packet with correct 2-hex checksum is received
- **THEN** the server transmits `+` acknowledgement and executes the command.

### Requirement: SEGGER RTT Telemetry Parser
The `agentla-debug-server` crate SHALL parse SEGGER RTT control blocks in target memory and extract circular up-buffer data without data loss.

#### Scenario: Finding RTT signature in RAM buffer
- **WHEN** scanning a simulated SRAM block containing `"SEGGER RTT\0\0\0\0\0\0"`
- **THEN** the parser successfully resolves buffer descriptors, sizes, and read/write offsets.
