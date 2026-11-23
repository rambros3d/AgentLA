# mcp-hardware-control-plane Specification

## Purpose
Provides a complete, self-contained Model Context Protocol (MCP) tool and resource surface covering LittleFS filesystem operations, system lifecycle control, mixed-signal instruments, and PWM hardware on AgentLA.

## Requirements

### Requirement: LittleFS Filesystem Control via MCP
The MCP server SHALL provide tools `fs_list`, `fs_read`, `fs_write`, `fs_stat`, `fs_mkdir`, `fs_remove`, and `fs_storage_stats` to inspect, read, and write files in `USER_FS` and `SYSTEM_FS` without requiring HTTP REST calls.

#### Scenario: Reading a directory listing via MCP
- **WHEN** client invokes MCP tool `fs_list` with `{"root": "user", "path": "/"}`
- **THEN** MCP server returns JSON array of directory entries with names, types, and byte sizes.

#### Scenario: Staging an OTA binary chunk via MCP
- **WHEN** client invokes MCP tool `fs_write` with `{"root": "user", "path": "/ota/update.bin", "hex_data": "41474C41", "append": false}`
- **THEN** MCP server writes bytes to `/ota/update.bin` in `USER_FS` and returns success status with bytes written.

### Requirement: System Lifecycle and Clock Control via MCP
The MCP server SHALL provide tools `system_info`, `system_reboot`, `system_bootsel`, `system_clock_set`, and `system_partitions` to inspect hardware vitals and control MCU lifecycle.

#### Scenario: Querying system partitions via MCP
- **WHEN** client invokes MCP tool `system_partitions`
- **THEN** MCP server returns layout of flash partitions including offsets, capacities, and active roles.

#### Scenario: Soft reset via MCP
- **WHEN** client invokes MCP tool `system_reboot` with `{"mode": "soft"}`
- **THEN** MCP server acknowledges with `{ "status": "rebooting" }` and triggers an MCU reset.

### Requirement: Mixed-Signal Instruments via MCP
The MCP server SHALL provide tools `oscilloscope_capture`, `generator_start`, `generator_stop`, `glitch_arm`, and `glitch_disarm` for analog signal acquisition and waveform generation.

#### Scenario: Capturing oscilloscope frame via MCP
- **WHEN** client invokes MCP tool `oscilloscope_capture` with `{"pin": 26, "samples": 128}`
- **THEN** MCP server returns sampled voltage values with computed Vpp, Vrms, and signal frequency metrics.

### Requirement: Hardware PWM Subsystem Control via MCP
The MCP server SHALL provide tools `pwm_claim`, `pwm_set`, and `pwm_release` for direct RP2350 PWM channel allocation and duty-cycle control.

#### Scenario: Setting PWM duty cycle via MCP
- **WHEN** client invokes MCP tool `pwm_set` with `{"pin": 25, "duty_pct": 50}`
- **THEN** MCP server configures the PWM slice for pin 25 to 50% duty cycle and returns success.
