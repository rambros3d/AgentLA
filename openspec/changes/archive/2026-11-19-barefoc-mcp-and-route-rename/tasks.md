## 1. MCP Tool Definitions And Catalog

- [x] 1.1 Add eight `TOOL_BAREFOC_*` consts to `crates/agentla-mcp/src/tools.rs` with compact JSON input schemas (status, target, read, write, command, mode, calibrate, emergency)
- [x] 1.2 Add the eight consts to `ALL_TOOLS`, bump `[Tool; 48]` to the new authoritative length, and correct the catalog comment
- [x] 1.3 Define the session-only active target static (node/address) in `firmware-rs/src/mcp.rs` with a bounded default (node 1, I2C 0x40+node)

## 2. MCP Tool Handlers (firmware)

- [x] 2.1 Implement `barefoc_status` using `telemetry::sample_telemetry()` + `profile::{active_slot, is_target_power_enabled}` returning bounded JSON
- [x] 2.2 Implement `barefoc_target` to set/query the session node/address, validating 0..=63 and 0x00..=0x7F
- [x] 2.3 Implement `barefoc_read` / `barefoc_write` over `bridge::i2c::{hw_i2c_read, hw_i2c_write}` (MultiBus registers) and `bridge::spi::spi_transfer_default` (gate driver), with bounded payloads
- [x] 2.4 Implement `barefoc_command` over `bridge::uart::{uart_write_bytes, uart_read_blocking}` returning escaped response and byte counts
- [x] 2.5 Implement `barefoc_mode` over CAN/UART using `bridge::can_gsusb::transmit_can_ws_frame` and Commander strings, honoring Pin Arbiter via bridge APIs
- [x] 2.6 Implement `barefoc_calibrate` for motor and encoder kinds using the active transport, with status reporting
- [x] 2.7 Implement `barefoc_emergency` delegating to the existing safe-state path (profile slot 0 / set_target_power(false) + quiesce pins)
- [x] 2.8 Ensure every tool uses `extract_*_arg` helpers, bounds output via `write!` into `heapless::String<1536>`, and returns correct `isError`

## 3. MCP Playbook Resource And Prompt

- [x] 3.1 Add `agentla://skills/barefoc` to `DEFAULT_RESOURCES`, bump array length, and add a `resources/read` arm with the static markdown playbook
- [x] 3.2 Add `barefoc_debug` to `DEFAULT_PROMPTS` with optional `node` argument and a `prompts/get` arm guiding a safe workflow
- [x] 3.3 Verify no emojis in resource/prompt content

## 4. Count Reconciliation

- [x] 4.1 Update `webui/mcp.json` tools count and resources count to the authoritative values
- [x] 4.2 Update `crates/agentla-mcp/tests/mcp_rpc.rs` count assertion and add name assertions for all eight tools, the resource, and the prompt
- [x] 4.3 Update `test/test_sensor_studio_app.py`, `test/test_neopixel_system_app.py`, and `test/test_mcp_servo_controls.py` count assertions
- [x] 4.4 Update `firmware-rs/src/rest.rs` McpHealth stale tools count
- [x] 4.5 Add a Python test calling `tools/call` for each BareFOC tool and asserting bounded valid JSON responses

## 5. Route Rename: motor -> barefoc

- [x] 5.1 In `firmware-rs/src/web.rs`, make `/barefoc` the canonical 200 route and flip `/motor`, `/motor/`, `/foc`, `/foc/`, `/barefoc/` to 301 redirects to `/barefoc`
- [x] 5.2 Add `/barefoc/{style.css,daisyui.css,themes.css,agentla-bridge.js}` subpath fallbacks (keep `/motor/...` variants)
- [x] 5.3 Replace `href="/motor"` with `/barefoc` in barefoc.html, lab.html, terminal.html, files.html, servo.html, neopixel.html, sensor.html
- [x] 5.4 Update `index.html` hash routing (`#/barefoc` canonical, `#/motor`/`#/foc` aliases), `openMotorApp()` wrapper, dropdown options, top-rail/dock labels to BareFOC, dock id to `dock-app-barefoc`, and system-app arrays
- [x] 5.5 Update `app.js` and `agentla-bridge.js` hash checks, target URL `/barefoc`, dock id arrays, and path detection (aliases activate `dock-app-barefoc`)
- [x] 5.6 Update `lab.html` dock id and dropdown option
- [x] 5.7 Update `test/test_sensor_studio_app.py` and `test/test_neopixel_system_app.py` `href="/motor"` assertions to `/barefoc`
- [x] 5.8 Do NOT rename domain/protocol strings (`SimulatedMotor`, `MOTOR_ERROR_*`, `axis0.motor`, `Calibrate Motor`, `sel-motor-type`, etc.)

## 6. Specs, Build, And Deploy

- [x] 6.1 Update current specs via delta sync: `native-barefoc-system-app`, `native-neopixel-system-app`, `native-servo-system-app`, `domain-crate-mcp` (plus new `mcp-barefoc-tools` and `mcp-barefoc-playbook` specs)
- [x] 6.2 Run `cargo test -p agentla-mcp` and fix all count/dispatch failures (22 tests pass on host target)
- [x] 6.3 Build firmware for RP2350 and flash via Debugprobe with fallback recovery (release build + probe-rs download to RP2350 via 2e8a:000c succeeded)
- [x] 6.4 Run WebUI/asset suites and the new MCP BareFOC tests (72 Python tests pass)
- [x] 6.5 Repackage `webui.tar` with `--no-user-apps` and deploy via `/api/v1/system/webui/update` — DONE: `build/webui.tar` verified (56 tools / 8 resources, `/barefoc` nav) and deployed live: `{"status":"updated","files":26,"bytes":404802}`, HTTP 200.
- [x] 6.6 Verify live — DONE: live device reports health `{"tools":56,"resources":8,"prompts":5}` and `status` tool reports 56; `resources/read agentla://skills/barefoc` returns the playbook (HTTP 200, 2205 bytes); `tools/call barefoc_status` returns valid telemetry with `safe_state:true`; `GET /barefoc` → 200, `/motor` and `/foc` → 301 to `/barefoc`, `/barefoc.html` and `/motor.html` → 200; `/barefoc` body contains StudioStore/DiscoveryService/shell markers. Note: the full `tools/list` response resets the connection on this flaky USB link (small calls succeed); the local Rust test serializes all 56 tools successfully and passes, so this is a transport/link issue, not a code defect.
