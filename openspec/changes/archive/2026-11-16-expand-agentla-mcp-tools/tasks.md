## 1. Tool Schemas & Definitions in agentla-mcp

- [x] 1.1 Define `TOOL_APP_LIST` and `TOOL_APP_INSTALL` constants with input schemas in `crates/agentla-mcp/src/tools.rs`
- [x] 1.2 Define `TOOL_POWER_CONTROL`, `TOOL_POWER_CYCLE`, and `TOOL_EMERGENCY_SAFE_STATE` constants in `crates/agentla-mcp/src/tools.rs`
- [x] 1.3 Define `TOOL_PROFILE_LIST`, `TOOL_PROFILE_APPLY`, and `TOOL_SWO_READ` constants in `crates/agentla-mcp/src/tools.rs`
- [x] 1.4 Register all 7 new tools in `agentla_mcp::tools` export list and verify `cargo test --package agentla-mcp`

## 2. Firmware Dispatch Handlers

- [x] 2.1 Implement `app_list` handler in `firmware-rs/src/mcp.rs` leveraging `crate::apps::list_apps_json`
- [x] 2.2 Implement `app_install` handler in `firmware-rs/src/mcp.rs` mapping catalog identifiers to cached packages
- [x] 2.3 Implement `power_control` and `power_cycle` handlers in `firmware-rs/src/mcp.rs` linking to profile power switch and voltage regulation
- [x] 2.4 Implement `emergency_safe_state` handler in `firmware-rs/src/mcp.rs` executing atomic pin release and power disconnect
- [x] 2.5 Implement `profile_list` and `profile_apply` handlers in `firmware-rs/src/mcp.rs` mapping to `crate::profile::apply_profile`
- [x] 2.6 Implement `swo_read` handler in `firmware-rs/src/mcp.rs` draining captured SWO bytes

## 3. WebUI & Manifest Integration

- [x] 3.1 Update `webui/mcp.json` tool count and schema documentation
- [x] 3.2 Update `firmware-rs/src/device_html.rs` or device diagnostics page tool count
- [x] 3.3 Add unit tests verifying `tools/list` returns all 36 tools without allocation
- [x] 3.4 Verify that `app_uninstall` returns error -32601 (method/tool not found)
