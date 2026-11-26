# Tasks: `i3c-private-write-read-rx-len`

- [x] 1. Document `I3cBulkHeader.status` request `rx_len` convention for `I3C_OP_PRIVATE_WRITE_READ` in `crates/agentla-bridges/src/i3c_bulk.rs` and add unit test verifying header round-trip.
- [x] 2. Update `firmware-rs/src/bridge/i3c.rs` (`I3C_OP_PRIVATE_WRITE_READ` handler) to set `cmd.rx_len` from `frame.header.status` (`0` -> `128`, otherwise `.min(128)`).
- [x] 3. Update `sdk/agentla/i3c.py` (`_transceive` / `private_write_read`) to pass `read_len` (`1..=128`) in header byte 7.
- [x] 4. Run `cargo test` on `agentla-bridges`, build `firmware-rs`, and verify against FishyServo `0x40` (host tests + release build verified; hardware flash deferred per caller).
