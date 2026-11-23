# Tasks

## 1. REST Endpoint Gating & Structured Warning Payloads

- [x] 1.1 Add `HTTP_PROGRAMMER_ENABLED: bool = false` and `HTTP_ADAPTERS_ENABLED: bool = false` build flags and warning response helpers in `firmware-rs/src/rest.rs` and verify clean build
- [x] 1.2 Gate all `/api/v1/programmer/*` endpoints in `firmware-rs/src/rest.rs` (`target`, `reset`, `read`, `write`, `rtt/*`, `swo/*`) to return `HTTP_PROGRAMMER_DISABLED_BODY`
- [x] 1.3 Gate all `/api/v1/bridge/*` and `/api/v1/bus/*` endpoints in `firmware-rs/src/rest.rs` (`i2c`, `i3c`, `spi`, `uart`, `can`) to return `HTTP_ADAPTERS_DISABLED_BODY`
- [x] 1.4 Update MCP health endpoint in `firmware-rs/src/rest.rs` to report tool count of 58 and verify with `pytest test/test_usb_adapters_mcp.py`

## 2. Firmware MCP Handler Redirection

- [x] 2.1 Update `firmware-rs/src/mcp.rs` tool handlers (`programmer_flash`, `programmer_unbrick`, `programmer_read`) to return structured USB redirection warnings
- [x] 2.2 Verify firmware compilation passes with `cargo check -p firmware-rs`

## 3. MCP Server Resources & Prompts Update

- [x] 3.1 Update `agentla://skills/programmer` and `agentla://skills/dev` skill text in `crates/agentla-mcp/src/server.rs` to guide agents to direct host USB tooling
- [x] 3.2 Update `dev_workflow` and `program_target` prompt templates in `crates/agentla-mcp/src/server.rs` to instruct agents to use USB debug probe tools (`probe-rs`, `pyocd`, `openocd`)
- [x] 3.3 Verify MCP crate tests pass with `cargo test -p agentla-mcp --target x86_64-unknown-linux-gnu`

## 4. Verification & Validation

- [x] 4.1 Run all unit and integration tests across `agentla-mcp` and Python test suites (`test_usb_adapters.py`, `test_usb_adapters_mcp.py`)
- [x] 4.2 Verify zero-emoji compliance and strict `#![no_std]` zero-heap standards across all modified files
