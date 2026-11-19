---
name: agentla-barefoc
description: Agentic debugging of external bareFOC/SimpleFOC motor targets via AgentLA's barefoc_* MCP tools: telemetry, MultiBus/SPI registers, Commander commands, mode/enable, calibration, error decode, and safe-state teardown.
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  tools_required: ["agentla-mcp"]
---

# BareFOC Agentic Debug Specialist

FOC loops run on the EXTERNAL target; AgentLA bridges I2C/SPI/UART/CAN only.
All tools are no_std, zero-heap, bounded.

## Workflow (safety first)
1. `barefoc_status` — check rail, current, overcurrent, safe_state.
2. `barefoc_target {"node":1,"addr":65}` — session-only target (default 0x40+node).
3. `i2c_scan` / `can_sniff` — confirm the target.
4. `barefoc_read` / `barefoc_write` — bounded MultiBus (I2C) or DRV83xx (SPI) registers (1-32 bytes); NACK means stop.
5. `barefoc_command {"command":"M1"}` — Commander status; `barefoc_mode {"mode":"idle"}` before enabling.
6. `barefoc_calibrate {"kind":"motor"|"encoder"}` — do not move the shaft.
7. Decode `MOTOR_ERROR_*` / SimpleFOC errors; tune PID via registers or `V P`/`V I`/`C P`.
8. `barefoc_emergency`, then `barefoc_status` to verify safe_state.

## Tools
`barefoc_status`, `barefoc_target`, `barefoc_read`, `barefoc_write`,
`barefoc_command`, `barefoc_mode`, `barefoc_calibrate`, `barefoc_emergency`.

Supporting: `i2c_scan`, `spi_transfer`, `uart_command`, `can_send`, `can_sniff`,
`sensor_poll`, `sniff_bus`, `logic_analyzer_*`, `power_control`, `profile_apply`,
`target_diagnose`, `status`.

## MultiBus registers (I2C 0x40+node)
0x01 TARGET(f32), 0x04 ENABLE, 0x05 MODE (0 torque/1 vel/2 angle/3 vel-open/4 angle-open),
0x09 ANGLE, 0x11 VELOCITY, 0x22 CURRENT_Q, 0x30-32 VEL PID P/I/D, 0x36 ANG P,
0x50 VOLTAGE_LIMIT, 0x52 VELOCITY_LIMIT.

Commander: `M1` status, `M1 C <mode>`, `M1 V <rad/s>`, `M1 A <rad>`, `M1 E 0/1`, `M1 W`.

## WebUI
Canonical `http://10.10.10.10/barefoc` (`/motor`, `/foc` are 301 aliases).
ODrive-GUI-style dashboards, parameter explorer, multi-axis footer, wizard, scope.

## Pins
Programmer 0-9 (SWD 4-5), Adapter 10-19 (I2C 10-11, UART 12-13), Analyzer 20-29.
Adapter+Programmer conflict on 0-1/4-5 — serialize. Use `pins_claim`/`pins_release`.
External 3.3 V, common GND, 4.7 kΩ I2C pull-ups for 400 kHz.

## Caveats
Target selection is session-only; tool output bounded to 1536 bytes; H5 SWD flash
limited — keep pyocd/probe-rs. Playbook resource: `agentla://skills/barefoc`;
prompt: `barefoc_debug`. Full doc: bareFOC `AGENTLA-USAGE.md`.
