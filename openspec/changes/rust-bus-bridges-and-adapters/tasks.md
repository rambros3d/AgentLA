## 1. Scaffolding

- [x] 1.1 Create `bridge.rs` module with feature-state hooks - compile clean
- [x] 1.2 Wire `mod bridge` in `main.rs`
- [ ] 1.3 Stub REST endpoints for all bridge enables (`/api/v1/bridge/i2c/{on,off`, etc.)

## 2. Proof-of-concept MCP2221 I2C

- [ ] 2.1 Implement handshake/read/write command parsers on VCOM
- [ ] 2.2 Claim I2C flex pins and open adapter read/write (500us max)
- [ ] 2.3 Bridge reset and pin release on disconnect

## 3. Verification

- [x] 3.1 `cargo build --release` clean
- [ ] 3.2 Real-device test: enumerate a sensor and basic read worked
