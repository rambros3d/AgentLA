# i3c-bridge Specification

## Purpose

TBD - created by archiving change i3c-private-write-read-rx-len. Update Purpose after archive.

## Requirements

### Requirement: Bounded RX Length on `I3C_OP_PRIVATE_WRITE_READ`
`I3C_OP_PRIVATE_WRITE_READ` (`0x09`) SHALL honor the requested read length (`1..=128` bytes) encoded in request header byte 7 (`I3cBulkHeader.status`), falling back to `128` bytes when byte 7 is `0`.

#### Scenario: 4-byte register read clocks exactly 4 bytes
- **WHEN** `AgentLaI3c.private_write_read(0x40, reg_bytes, read_len=4)` is called
- **THEN** the request header encodes `4` in byte 7
- **AND** `firmware-rs` sets `cmd.rx_len = 4` and clocks `4` bytes from the target instead of `128`

#### Scenario: Legacy zero header byte preserves 128-byte fallback
- **WHEN** a request frame for `I3C_OP_PRIVATE_WRITE_READ` arrives with header byte 7 equal to `0`
- **THEN** `firmware-rs` sets `cmd.rx_len = 128`
