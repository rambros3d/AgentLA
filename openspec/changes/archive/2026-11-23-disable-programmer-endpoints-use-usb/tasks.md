# Tasks

## 1. Firmware MCP Programmer Tool Gating

- [x] 1.1 Gate `probe_target`, `programmer_reset`, `target_diagnose`, `inspect_firmware`, and `swo_read` behind `HTTP_PROGRAMMER_ENABLED` in `firmware-rs/src/mcp.rs` to return `HTTP_PROGRAMMER_DISABLED_BODY`
- [x] 1.2 Update `HTTP_PROGRAMMER_DISABLED_BODY` in `firmware-rs/src/rest.rs` to emphasize that both API and MCP programmer endpoints are disabled in favor of direct USB interfaces (`CMSIS-DAP`, `ST-Link`, `WCH-Link`)

## 2. MCP Tools Schema & Guidance Update

- [x] 2.1 Update tool descriptions for `probe_target`, `programmer_read`, `programmer_flash`, `programmer_reset`, `programmer_unbrick`, `target_diagnose`, `inspect_firmware`, and `swo_read` in `crates/agentla-mcp/src/tools.rs`
- [x] 2.2 Update skills and prompts in `crates/agentla-mcp/src/server.rs` (`agentla://skills/programmer`, `agentla://skills/dev`, `agentla://skills/debug`, `dev_workflow`, `debug_target`, `program_target`) to mandate host USB debug probe toolchains (`probe-rs`, `pyocd`, `openocd`)

## 3. Verification & Deployment

- [x] 3.1 Verify unit tests pass with `cargo test -p agentla-mcp --target x86_64-unknown-linux-gnu` and Python tests pass with `pytest test/test_usb_adapters_mcp.py`
- [x] 3.2 Verify firmware compilation passes with `cargo check -p firmware-rs`
- [x] 3.3 Build release firmware and deploy to AgentLA via SWD Debugprobe (`probe-rs download --chip RP235x --probe 2e8a:000c --verify`) followed by hardware reset
- [x] 3.4 Verify live MCP tool calls on the physical device (`10.10.10.10`) confirm programmer tools return the USB redirection warning
- [x] 3.5 Ensure strict `#![no_std]` zero-heap compliance and zero-emoji compliance across all modified files
