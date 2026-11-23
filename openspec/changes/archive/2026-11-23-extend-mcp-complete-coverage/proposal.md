# Proposal

## Why

AI agents currently fall back to the HTTP REST API because critical device operations—such as LittleFS filesystem inspection/staging, system lifecycle reboots, mixed-signal oscilloscope captures, signal generation, and raw PWM timers—are missing from the Model Context Protocol (MCP) tool catalog. Extending the MCP surface with full control-plane coverage empowers agents to operate purely through structured, typed JSON-RPC tools with zero shell-script or HTTP REST dependencies.

## What Changes

- **Add LittleFS Filesystem MCP Tools**: Introduce `fs_list`, `fs_read`, `fs_write`, `fs_stat`, `fs_mkdir`, `fs_remove`, and `fs_storage_stats` to inspect and modify files in `USER_FS` and `SYSTEM_FS` (e.g. for staging OTA update files and managing configurations).
- **Add System Lifecycle & Hardware MCP Tools**: Introduce `system_info`, `system_reboot`, `system_bootsel`, `system_clock_set`, and `system_partitions` to allow agents to inspect partitions, adjust system clocks, and trigger controlled resets without HTTP routes.
- **Add Mixed-Signal Instrument MCP Tools**: Introduce `oscilloscope_capture`, `generator_start`, `generator_stop`, `glitch_arm`, and `glitch_disarm` to provide hardware signal acquisition, waveform generation, and clock/power glitch injection directly via MCP.
- **Add General PWM Subsystem MCP Tools**: Introduce `pwm_claim`, `pwm_set`, and `pwm_release` for direct RP2350 PWM slice frequency and duty-cycle control.
- **Add Enhanced MCP Resources and Prompts**: Expose dynamic hardware resources (`agentla://fs/stats`, `agentla://partitions`) and playbooks (`ota_update_workflow`, `analog_test_workflow`).
- **Synchronize Catalog Counts**: Keep authoritative tool count synchronized across `crates/agentla-mcp`, `firmware-rs/src/mcp.rs`, `firmware-rs/src/rest.rs`, `webui/mcp.json`, and all test suites.

## Capabilities

### New Capabilities
- `mcp-hardware-control-plane`: Complete MCP tool surface for filesystem operations, system lifecycle resets, mixed-signal instruments (oscilloscope/generator), and PWM hardware.

### Modified Capabilities
- `domain-crate-mcp`: Updates authoritative catalog counts and test assertions to reflect the expanded toolset.

## Impact

- `crates/agentla-mcp/src/tools.rs`: Defines new tool schemas with strict parameter typing, validation, and documentation.
- `crates/agentla-mcp/src/server.rs`: Implements new resources and prompts.
- `firmware-rs/src/mcp.rs`: Implements tool execution dispatch connecting into `crate::storage`, `crate::fs_http`, `crate::instruments`, and `crate::servo`.
- `firmware-rs/src/rest.rs` & `webui/mcp.json`: Updates health and manifest tool counts.
- `crates/agentla-mcp/tests/mcp_rpc.rs` & `test/test_usb_adapters_mcp.py`: Verifies all tool schemas, dispatch arms, and catalog counts pass.
