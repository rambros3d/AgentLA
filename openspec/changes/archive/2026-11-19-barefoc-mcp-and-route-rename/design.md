## Context

AgentLA exposes a `#![no_std]` MCP server (`crates/agentla-mcp`, JSON-RPC 2.0 over `POST /mcp`, discovery via `/mcp/sse` and `/mcp.json`). Its `ALL_TOOLS` array currently has 48 entries while several tests/manifests claim 49/45/40 — an existing inconsistency. There is no FOC-specific tool. BareFOC Studio (`webui/barefoc.html`) is a SYSTEM_FS WebUI app served at canonical `/motor` with `/foc` and `/barefoc` redirects; it talks to **external** FOC targets over CAN, UART (Commander ASCII), Fibre RPC, and MultiBus I2C/SPI via the `window.AgentLA` bridge SDK. The AGENTS.md rules require strict no_std/zero-heap firmware, Pin Arbiter ownership for pin access, three-image flash layout (untouched here), zero emojis, and specific UI geometry.

## Goals / Non-Goals

**Goals:**
- Give agents MCP-callable BareFOC capabilities that wrap existing bridges to read/command/calibrate an external FOC target safely.
- Provide a static BareFOC playbook resource + prompt so an agent has a repeatable debugging workflow.
- Make `/barefoc` the canonical WebUI route and `/motor` a compatibility redirect; update labels/links/hash routing.
- Reconcile all stale tool/resource counts to one authoritative source.

**Non-Goals:**
- No FOC loop, field-oriented control, or PID computation running on the AgentLA probe (external targets only).
- No new Rust crate, no new transport, no change to `/mcp` protocol methods or SSE behavior.
- No flash-layout/OTA changes; no change to servo/neopixel/sensor tool behavior.

## Decisions

**D1 — Tools wrap bridges, not a firmware FOC engine.** Each `barefoc_*` tool calls existing primitives in `firmware-rs/src/mcp.rs`: `telemetry::sample_telemetry()` (fresh ADC), `bridge::i2c::hw_i2c_*` (MultiBus registers at 0x40+node), `bridge::spi::spi_transfer_default` (DRV83xx gate driver), `bridge::uart::uart_write_bytes`/`uart_read_blocking` (Commander), `bridge::can_gsusb::{transmit_can_ws_frame, pop_can_rx_frame}` (ODrive CAN), `profile::{set_target_power, apply_profile_sync}`. Rationale: FOC runs on external targets; the probe only bridges. Alternative (port the whole FOC stack to the probe) rejected as out of scope.

**D2 — Stateless tools with explicit target args.** There is no persistent FOC state on the probe (only atomics for power/voltage/overcurrent and profile slots). Tools accept `node` (default 1), `addr` (default 0x40+node), `reg`, `bus`, etc. per call. Optional `barefoc_target` stores a chosen node/address in a small `static` Mutex in `firmware-rs/src/mcp.rs` (following `profile.rs`'s `ACTIVE_PROFILE_*` pattern) as a convenience default. Rationale: zero-heap and no global FOC parameter store; callers remain explicit and safe.

**D3 — Tool set (8 tools).** `barefoc_status`, `barefoc_target`, `barefoc_read`, `barefoc_write`, `barefoc_command`, `barefoc_mode`, `barefoc_calibrate`, `barefoc_emergency`. Each has a hand-written JSON input schema in `tools.rs`, a match arm in `invoke_tool` using `extract_string_arg`/`extract_u32_arg`/`extract_i32_arg`/`extract_bool_arg`, and bounded `write!` output into `heapless::String<1536>`. `barefoc_emergency` delegates to the existing safe-state path (profile slot 0 / `set_target_power(false)` + quiesce pins) rather than duplicating it.

**D4 — Resource + prompt are static.** Add `agentla://skills/barefoc` to `DEFAULT_RESOURCES` ([Resource; 7]→[8]) with a `resources/read` arm containing a static markdown playbook (safety-first, discovery, telemetry, register access, calibration, error decode, teardown). Add `barefoc_debug` to `DEFAULT_PROMPTS` ([Prompt; 4]→[5]) with an optional `node` argument. Rationale: `McpServer` has no resource handler, so live data must come from tools; playbook content is inherently static.

**D5 — Single authoritative count.** Set `ALL_TOOLS` to `56` (48 + 8). Update `webui/mcp.json` tools count to 56, `crates/agentla-mcp/tests/mcp_rpc.rs:89` to 56, `test/test_sensor_studio_app.py` (49→56), `test/test_neopixel_system_app.py` (45/49→56), `test/test_mcp_servo_controls.py` (40/45→56), and `firmware-rs/src/rest.rs:1988` health `"tools":56`. Resources count → 8 in `mcp.json`; `test_mcp_servo_controls.py` resources 2→8. Rationale: stop the drift; a single value derived from the actual arrays.

**D6 — Route swap.** In `firmware-rs/src/web.rs`: `/barefoc` serves `barefoc.html` (200), `/barefoc/` → 301 `/barefoc`; `/motor`, `/motor/`, `/foc`, `/foc/` → 301 `/barefoc`; keep `/barefoc.html` and `/motor.html` serving the asset; add `/barefoc/{style.css,daisyui.css,themes.css,agentla-bridge.js}` subpath fallbacks (keep `/motor/...` variants for compatibility). Rationale: `barefoc` is already the asset/catalog/packaging name; `/motor` becomes a backwards-compatible alias.

**D7 — WebUI references.** Replace `href="/motor"`→`href="/barefoc"` in all system pages and `barefoc.html`; `index.html` hash route canonical `#/barefoc` (keep `#/motor`/`#/foc` aliases), `openMotorApp()` → `openBarefocApp()` (or keep wrapper), dropdown options to `/barefoc`, top-rail/dock labels to "BareFOC", dock id `dock-app-motor`→`dock-app-barefoc`, arrays `'motor'`→`'barefoc'`. Same in `app.js` and `agentla-bridge.js` (path detection keeps aliases but activates `dock-app-barefoc`). Domain strings (`SimulatedMotor`, `MOTOR_ERROR_*`, `axis0.motor`, "Calibrate Motor", `sel-motor-type`) are NOT renamed.

**D8 — Specs.** Update current `native-barefoc-system-app/spec.md` route/nav requirements; `native-neopixel-system-app` and `native-servo-system-app` reference `/motor` in scenarios — update via delta. Archived changes are left untouched.

**D9 — Verification.** Extend `crates/agentla-mcp/tests/mcp_rpc.rs` with tool names + count + resource/prompt presence; add a Python test that calls `tools/call` for each BareFOC tool against the live device (or a stubbed handler) and asserts bounded JSON responses; run the full WebUI/asset suites; rebuild firmware with `cargo build` and repackage/deploy WebUI. Debugprobe is the recovery fallback if firmware update fails.

## Risks / Trade-offs

- [Adding 8 tools grows `ALL_TOOLS`; firmware binary/static strings grow → Mitigation: schemas are compact `&'static str`; 1536-byte output bound unchanged; verify build size/flash headroom.]
- [Direct I2C/SPI/UART/CAN bus access could disturb other subsystems → Mitigation: require `pins_claim` for direct pins where applicable, use existing bridge drivers that already manage ownership, and keep `barefoc_emergency` as the single safe teardown path.]
- [External targets vary (SimpleFOC vs ODrive vs MultiBus) → Mitigation: tools are transport-generic with explicit args; playbook documents per-target command sets; no assumption of a single wire format.]
- [Stale counts exist in many files → Mitigation: D5 enumerates every site; a test asserts the single authoritative value.]
- [Route rename breaks bookmarks to `/motor` → Mitigation: 301 redirects preserve old links; hash aliases retained; tests assert both canonical and alias behavior.]
- [Renaming dock ids could break CSS/JS selectors → Mitigation: update all `dock-app-${sysId}` arrays and `sysStyle()` mapping together; verify with asset tests + browser screenshot.]

## Migration Plan

1. Add tool consts + `invoke_tool` arms; bump counts; add resource/prompt; run `cargo test -p agentla-mcp` and fix count assertions.
2. Build firmware (`cargo build` for the target) and flash via the established flow (Debugprobe fallback available).
3. Swap WebUI routes and references; update tests; run WebUI/asset suites; repackage `webui.tar` with `--no-user-apps` and deploy.
4. Verify live: `tools/list` advertises BareFOC tools; `resources/read agentla://skills/barefoc` returns the playbook; `/barefoc` returns 200 and `/motor` 301s; browser smoke test of the app.

## Open Questions

- Keep the `openMotorApp()` function name as an alias for compatibility, or fully rename to `openBarefocApp()`? Design assumes both exist (wrapper) to minimize breakage.
- Should `barefoc_target` persist the chosen node/address across reboots, or be session-only? Design assumes session-only static (no LittleFS state).
