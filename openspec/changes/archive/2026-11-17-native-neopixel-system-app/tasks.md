## 1. Web Asset Migration & Navigation Integration

- [x] 1.1 Copy and adapt `apps/neopixel/index.html` to `webui/neopixel.html` as a first-party in-ROM system asset
- [x] 1.2 Update internal asset paths to reference root-level canonical system styles and scripts (`/daisyui.css`, `/style.css`, `/themes.css`, `/agentla-bridge.js`)
- [x] 1.3 Add canonical top-rail navigation to `webui/neopixel.html` matching workbench layout (Home, Scope, Terminal, Files, Motor, Servo, NeoPixel)
- [x] 1.4 Add NeoPixel navigation link to `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, `webui/barefoc.html`, and `webui/servo.html`
- [x] 1.5 Formalize NeoPixel item in floating dock and application drawer in `webui/agentla-bridge.js` and `webui/app.js` using Solar bold duotone filters icon
- [x] 1.6 Ensure surface-only active state highlights with zero CSS dot indicators
- [x] 1.7 Configure deep linking and hash routing for `/neopixel` and `#/neopixel`

## 2. Firmware Route Registration

- [x] 2.1 Register `/neopixel`, `/neopixel/`, and `/neopixel.html` routes in `firmware-rs/src/web.rs` backed by `SystemWebFile::new("neopixel.html", "text/html")`
- [x] 2.2 Register 301 redirect alias for `/led` and `/led/` in `firmware-rs/src/web.rs` pointing to `/neopixel`
- [x] 2.3 Verify `firmware-rs` compiles cleanly `#![no_std]` targeting `thumbv8m.main-none-eabihf`

## 3. MCP Tool Declaration and Dispatch

- [x] 3.1 Declare `TOOL_NEOPIXEL_FILL`, `TOOL_NEOPIXEL_SET`, `TOOL_NEOPIXEL_CLEAR`, `TOOL_NEOPIXEL_PATTERN`, `TOOL_NEOPIXEL_ANALYZE`, and `TOOL_NEOPIXEL_TRANSMIT` in `crates/agentla-mcp/src/tools.rs`
- [x] 3.2 Add tools to `ALL_TOOLS` array in `crates/agentla-mcp/src/tools.rs`
- [x] 3.3 Implement JSON-RPC tool dispatch in `firmware-rs/src/mcp.rs` delegating to `crate::bridge::ws2812` and `crate::instruments::ws2812`
- [x] 3.4 Update `webui/mcp.json` tool count to 45
- [x] 3.5 Verify `cargo test -p agentla-mcp` passes cleanly

## 4. Packaging, Verification & Tests

- [x] 4.1 Update `tools/pack_webui.py` to include `neopixel` in `SYSTEM_APPS` and package `neopixel.html` into `build/webui.img` and `build/webui.tar`
- [x] 4.2 Validate asset size within `SYSTEM_FS` partition headroom
- [x] 4.3 Add automated pytest tests in `test/test_neopixel_system_app.py` verifying routes, dock icons, zero emojis, and MCP tool discovery
- [x] 4.4 Run all test suites to confirm clean execution
