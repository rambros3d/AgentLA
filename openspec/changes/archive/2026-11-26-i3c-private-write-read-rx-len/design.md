# Design: `i3c-private-write-read-rx-len`

## Wire Framing
`I3cBulkHeader` is 8 bytes (`<BBBBHBB`: `magic`, `opcode`, `flags`, `seq`, `payload_len`, `addr`, `status`).
On host-to-device requests, byte 7 (`status`) is always `0`. Reusing byte 7 as `req_rx_len` for `I3C_OP_PRIVATE_WRITE_READ` (`0x09`) keeps `I3C_HEADER_SIZE == 8` and preserves full wire compatibility:
- `status == 0`: legacy behavior (`cmd.rx_len = 128`).
- `1 <= status <= 128`: `cmd.rx_len = frame.header.status`.
- `status > 128`: clamped to `128` (`frame.header.status.min(128)`).

## Execution Path in `firmware-rs/src/bridge/i3c.rs`
`CmdKind::Transfer` with `data_len > 0` and `rx_len > 0` already executes `do_write()` (`W + STOP`), a `150_000`-cycle (~1 ms) turnaround delay, and `do_read(&h, cmd.addr, &mut r.rx[..cmd.rx_len as usize])` (`R + STOP`). Setting `cmd.rx_len` to the exact requested byte count (e.g. `1` for `STATUS`, `4` for `ANGLE`) ensures the read fits inside the STM32H5 `I3C2` 8-byte preloaded TX FIFO with zero TX underruns or `READ_WINDOW` overflows.
