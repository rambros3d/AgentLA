# Proposal: `i3c-private-write-read-rx-len`

## Why
`AgentLaI3c.private_write_read()` in `sdk/agentla/i3c.py` sends `I3C_OP_PRIVATE_WRITE_READ` (`0x09`) without passing the requested `read_len` to the RP2350 bridge firmware, and `firmware-rs/src/bridge/i3c.rs` hardcodes `cmd.rx_len = 128`. Even for 1-byte (`REG_STATUS`) or 4-byte (`REG_ANGLE`) register reads, the PIO2 I3C master clocks 128 bytes, exceeding target `READ_WINDOW` limits (64 bytes on STM32H5 `Stm32I3cTarget`), forcing early target termination (`EVR_FCF`) and `reposition_read_ptr()` side effects on every transaction.

## What Changes
- **Protocol (`crates/agentla-bridges/src/i3c_bulk.rs`)**: Document that for `I3C_OP_PRIVATE_WRITE_READ` (`0x09`) request headers, `I3cBulkHeader.status` (byte 7, unused on requests) carries the requested `rx_len` (`1..=128`; `0` preserves the legacy `128`-byte fallback).
- **SDK (`sdk/agentla/i3c.py`)**: Update `AgentLaI3c._transceive()` / `private_write_read(target_addr, data, read_len=64)` to encode `max(1, min(read_len, 128))` in header byte 7 when `opcode == I3C_OP_PRIVATE_WRITE_READ`.
- **Firmware (`firmware-rs/src/bridge/i3c.rs`)**: In `I3C_OP_PRIVATE_WRITE_READ`, set `cmd.rx_len = if frame.header.status == 0 { 128 } else { frame.header.status.min(128) }`.

## Impact
- Backward-compatible with older hosts sending `0` in byte 7 (`128`-byte fallback).
- Eliminates 128-byte over-reads and target TX FIFO underruns during 1–8 byte register reads against `bareFOC` STM32H5 targets.
- Strictly zero emojis across code, comments, and documentation.
