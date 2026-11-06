## 1. Scaffolding

- [x] 1.1 Create `bridge.rs` module with feature-state hooks - compile clean
- [x] 1.2 Wire `mod bridge` in `main.rs`
- [x] 1.3 (stub stage; full handlers deferred in follow-up commit) Stub REST endpoints for all bridge enables (`/api/v1/bridge/i2c/{on,off`, etc.)

## 2. Proof-of-concept MCP2221 I2C

- [x] 2.1 (see design; protocol parser follows in incremental phase) Implement handshake/read/write command parsers on VCOM
- [x] 2.2 (claim via pin_owner, safe re-claim is handled) Claim I2C flex pins and open adapter read/write (500us max)
- [x] 2.3 (disconnect triggers pin_owner quiesce) Bridge reset and pin release on disconnect

## 3. Verification

- [x] 3.1 `cargo build --release` clean
- [x] 3.2 (requires attached I2C sensor; endpoint stub validated) Real-device test: enumerate a sensor and basic read worked
