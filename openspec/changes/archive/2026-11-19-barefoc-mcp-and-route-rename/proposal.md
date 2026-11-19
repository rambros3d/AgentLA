## Why

BareFOC Studio is a powerful FOC workbench, but an agent cannot debug a motor without driving the WebUI: the MCP surface has no FOC-aware tools and the canonical route is `/motor`, which diverges from the `barefoc` naming used by the asset, catalog, and packaging tools. This change exposes BareFOC capabilities as first-class MCP tools and a playbook resource so an agent can read telemetry, poke registers, command modes, and run calibration on an external FOC target — and renames the canonical endpoint to `/barefoc` for consistency.

## What Changes

- **New BareFOC MCP tools** (wrapping existing bridge primitives; FOC loops remain on external targets):
  - `barefoc_status` — fresh target rail telemetry + active profile + safe-state status.
  - `barefoc_target` — set/query the active external target (transport, node/address) passed to subsequent tool calls.
  - `barefoc_read` / `barefoc_write` — read/write a MultiBus register (I2C default 0x40+node) or a raw SPI gate-driver register on the target.
  - `barefoc_command` — send a SimpleFOC Commander ASCII command over UART and return the response (e.g. `M1`, `M1 C 2`, `M1 V 10`).
  - `barefoc_mode` — command motion mode / torque sub-mode / enable on the target (maps to CAN/UART commands).
  - `barefoc_calibrate` — trigger motor or encoder calibration on the target and report status (ODrive requested-state or SimpleFOC alignment).
  - `barefoc_emergency` — one-call safe-state: disable inverter, zero target, High-Z pins, cut target power.
  - All tools are #![no_std], zero-heap, bounded output, and honor the Pin Arbiter contract (claim/release around direct pin access).
- **New MCP resource** `agentla://skills/barefoc` — a static "BareFOC Agentic Debug Playbook" (safety first, discovery, telemetry, register access, calibration, error decode, safe-state teardown).
- **New MCP prompt** `barefoc_debug` — guided agent workflow with an optional `node` argument.
- **Tool-count consistency fix**: `ALL_TOOLS`, `webui/mcp.json`, the Rust `mcp_rpc.rs` count assertion, the three Python count assertions, and the stale `McpHealth {"tools":14}` are reconciled to one authoritative number.
- **BREAKING (route)**: `/barefoc` becomes the canonical HTTP 200 route serving `barefoc.html`; `/motor`, `/motor/`, `/foc`, `/foc/`, `/barefoc/` become 301 redirects to `/barefoc`. Hash routes `#/barefoc` canonical with `#/motor`/`#/foc` aliases. Nav labels change from "Motor" to "BareFOC" for consistency. `/barefoc.html` and `/motor.html` keep serving the asset directly.
- **No new Rust crate**, no FOC loop on AgentLA, no flash-layout changes.

## Capabilities

### New Capabilities
- `mcp-barefoc-tools`: MCP tools that wrap existing hardware bridges to let an agent discover, read, command, calibrate, and safely tear down an external FOC motor target.
- `mcp-barefoc-playbook`: MCP resource and prompt giving agents a BareFOC-specific debugging playbook.

### Modified Capabilities
- `native-barefoc-system-app`: canonical route changes from `/motor` to `/barefoc`; nav/launcher/dock labels and hash routing updated; `/motor` retained as a redirect alias.
- `domain-crate-mcp`: authoritative MCP tool/resource/prompt catalog counts and registry are updated (no transport/protocol behavior change beyond the new entries).

## Impact

- **Code**: `crates/agentla-mcp/src/tools.rs` (tool defs + counts), `crates/agentla-mcp/src/server.rs` (resource/prompt registry + read/get arms), `firmware-rs/src/mcp.rs` (invoke_tool arms using `extract_*` + `bridge::{i2c,spi,uart,can}` + `telemetry`/`profile`), `firmware-rs/src/rest.rs` (health count), `firmware-rs/src/web.rs` (routes).
- **WebUI**: `webui/mcp.json` (counts), nav/link/dock/hash changes across `index.html`, `app.js`, `agentla-bridge.js`, `lab.html`, `barefoc.html`, `terminal.html`, `files.html`, `servo.html`, `neopixel.html`, `sensor.html`.
- **Tests**: `crates/agentla-mcp/tests/mcp_rpc.rs`, `test/test_sensor_studio_app.py`, `test/test_neopixel_system_app.py`, `test/test_mcp_servo_controls.py`, plus new MCP BareFOC tool tests; `href="/motor"` assertions updated to `/barefoc`.
- **Firmware rebuild required** (Rust changes) + WebUI repackage/deploy. Debugprobe available as recovery fallback. Flash layout and OTA bounds unchanged.
- **UI standards**: zero emojis, no CSS dot indicators, dock anchored bottom, centered launcher preserved.
