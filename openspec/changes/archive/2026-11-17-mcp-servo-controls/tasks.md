## 1. Tool Declaration in agentla-mcp

- [x] 1.1 Declare `TOOL_SERVO_ATTACH`, `TOOL_SERVO_WRITE_ANGLE`, `TOOL_SERVO_WRITE_US`, and `TOOL_SERVO_DETACH` in `crates/agentla-mcp/src/tools.rs` with JSON input schemas
- [x] 1.2 Add the 4 new tools to `ALL_TOOLS` array in `crates/agentla-mcp/src/tools.rs`
- [x] 1.3 Verify `cargo test -p agentla-mcp` passes and tool count is updated

## 2. Firmware Dispatch Handlers

- [x] 2.1 Implement `servo_attach` handler in `firmware-rs/src/mcp.rs` parsing `pin`, `min_us`, `max_us`, and `continuous`
- [x] 2.2 Implement `servo_write_angle` handler in `firmware-rs/src/mcp.rs` parsing `pin` and `angle`
- [x] 2.3 Implement `servo_write_us` handler in `firmware-rs/src/mcp.rs` parsing `pin` and `pulse_us`
- [x] 2.4 Implement `servo_detach` handler in `firmware-rs/src/mcp.rs` parsing `pin`
- [x] 2.5 Ensure error paths return `isError: true` with informative technical diagnostics

## 3. Manifest & Verification

- [x] 3.1 Update `webui/mcp.json` tool count to 40
- [x] 3.2 Add or update tests in `test/test_mcp_*.py` verifying tool discovery via `tools/list` and call behavior
- [x] 3.3 Verify `#![no_std]` compilation cleanly targeting `thumbv8m.main-none-eabihf`
