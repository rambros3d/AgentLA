# Design

## Context

See `proposal.md` for background and motivation.

AgentLA's `crates/agentla-mcp` currently defines 58 tools across general pins, servos, logic analyzer, target catalog, USB adapters, and BareFOC. However, operations involving LittleFS files, MCU reset/BOOTSEL, clock tuning, oscilloscope captures, waveform generation, and raw PWM timers were implemented only as HTTP REST endpoints (`firmware-rs/src/rest.rs`).

To make MCP the sole control plane for agents, these capabilities must be formally brought into `crates/agentla-mcp` and wired to their existing firmware implementations in `firmware-rs/src/mcp.rs`.

## Goals / Non-Goals

**Goals:**
- Add LittleFS tools: `fs_list`, `fs_read`, `fs_write`, `fs_stat`, `fs_mkdir`, `fs_remove`, `fs_storage_stats`.
- Add System lifecycle tools: `system_info`, `system_reboot`, `system_bootsel`, `system_clock_set`, `system_partitions`.
- Add Instrument tools: `oscilloscope_capture`, `generator_start`, `generator_stop`, `glitch_arm`, `glitch_disarm`.
- Add PWM tools: `pwm_claim`, `pwm_set`, `pwm_release`.
- Add dynamic resources (`agentla://fs/stats`, `agentla://partitions`) and playbooks (`ota_update_workflow`, `analog_test_workflow`).
- Maintain `#![no_std]` zero-heap compliance and zero-emoji rules across all crates.

**Non-Goals:**
- Removing the HTTP REST endpoints: Human web dashboards (Bento file manager, oscilloscope canvas) continue using HTTP/SSE.
- Streaming large binaries (>256 bytes per chunk) in single MCP calls: Files are read and written in bounded, safe chunks.

## Decisions

### 1. Hex-Encoded Bounded Chunking for Filesystem I/O
- **Decision**: `fs_write` accepts hex-encoded strings up to 256 decoded bytes per call. `fs_read` returns hex or text string bounded by length.
- **Rationale**: Keeps MCP tool requests and responses within `#![no_std]` stack buffer limits without heap allocation.

### 2. Direct Hardware Hookups in `firmware-rs/src/mcp.rs`
- **Decision**: Wire tools directly into existing subsystem modules:
  - Filesystem $\rightarrow$ `crate::storage::user_fs()`, `crate::storage::system_fs()`
  - System Lifecycle $\rightarrow$ `cortex_m::peripheral::SCB::sys_reset()`, `crate::system::reset_to_bootsel()`
  - Oscilloscope $\rightarrow$ `crate::instruments::oscilloscope`
  - Generator $\rightarrow$ `crate::instruments::generator`
  - PWM $\rightarrow$ `crate::servo::pwm_claim`, `pwm_set`, `pwm_release`
- **Rationale**: Reuses tested driver infrastructure without duplicating code.

### 3. Tool Count Synchronization
- **Decision**: Compute exact new tool count (~74 tools), update `crates/agentla-mcp/src/tools.rs`, `webui/mcp.json`, and `/mcp/health`.
- **Rationale**: Enforces single source of truth and ensures all unit and integration test assertions pass.

## Risks / Trade-offs

- **[Risk] Stack overflow in `#![no_std]` tool dispatcher during large file chunk serialization.**  
  → **Mitigation**: Bounded buffers (`heapless::String<512>`, `[u8; 256]`) with explicit length checks.
- **[Risk] Flash partition write violation during MCP `fs_write`.**  
  → **Mitigation**: Writes are scoped exclusively to mounted LittleFS filesystems (`USER_FS` and `SYSTEM_FS`); raw flash addresses are disallowed.
