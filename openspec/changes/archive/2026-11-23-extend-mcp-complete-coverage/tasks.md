# Tasks

## 1. Filesystem Tools in MCP (`crates/agentla-mcp` & `firmware-rs`)

- [x] 1.1 Define LittleFS tools (`TOOL_FS_LIST`, `TOOL_FS_READ`, `TOOL_FS_WRITE`, `TOOL_FS_STAT`, `TOOL_FS_MKDIR`, `TOOL_FS_REMOVE`, `TOOL_FS_STORAGE_STATS`) in `crates/agentla-mcp/src/tools.rs`
- [x] 1.2 Implement LittleFS tool execution dispatch in `firmware-rs/src/mcp.rs` interfacing with safe LittleFS handles in `crate::storage`

## 2. System Lifecycle Tools in MCP

- [x] 2.1 Define system lifecycle tools (`TOOL_SYSTEM_INFO`, `TOOL_SYSTEM_REBOOT`, `TOOL_SYSTEM_BOOTSEL`, `TOOL_SYSTEM_CLOCK_SET`, `TOOL_SYSTEM_PARTITIONS`) in `crates/agentla-mcp/src/tools.rs`
- [x] 2.2 Implement system lifecycle execution dispatch in `firmware-rs/src/mcp.rs` handling soft reboot, bootsel jump, and partition reporting

## 3. Mixed-Signal Instruments and PWM Tools in MCP

- [x] 3.1 Define instrument tools (`TOOL_OSCILLOSCOPE_CAPTURE`, `TOOL_GENERATOR_START`, `TOOL_GENERATOR_STOP`, `TOOL_GLITCH_ARM`, `TOOL_GLITCH_DISARM`) in `crates/agentla-mcp/src/tools.rs`
- [x] 3.2 Define PWM tools (`TOOL_PWM_CLAIM`, `TOOL_PWM_SET`, `TOOL_PWM_RELEASE`) in `crates/agentla-mcp/src/tools.rs`
- [x] 3.3 Implement instruments and PWM execution dispatch in `firmware-rs/src/mcp.rs` wiring into `crate::instruments` and `crate::servo`

## 4. Resources, Prompts, and Catalog Synchronization

- [x] 4.1 Define dynamic resources (`agentla://fs/stats`, `agentla://partitions`) in `crates/agentla-mcp/src/server.rs`
- [x] 4.2 Define interactive prompts (`ota_update_workflow`, `analog_test_workflow`) in `crates/agentla-mcp/src/server.rs`
- [x] 4.3 Synchronize authoritative tool count in `ALL_TOOLS`, `webui/mcp.json`, and `firmware-rs/src/rest.rs` `/mcp/health`

## 5. Verification & Testing

- [x] 5.1 Update unit tests in `crates/agentla-mcp/tests/mcp_rpc.rs` and verify all tests pass with `cargo test -p agentla-mcp --target x86_64-unknown-linux-gnu`
- [x] 5.2 Verify Python test suites pass with `pytest test/test_usb_adapters_mcp.py`
- [x] 5.3 Verify firmware compilation with `cargo check -p firmware-rs`
- [x] 5.4 Ensure strict zero-emoji compliance and `#![no_std]` zero-heap compliance across all files
