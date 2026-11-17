## 1. Web Asset Migration & Adaptation

- [x] 1.1 Copy and adapt `apps/servo/index.html` to `webui/servo.html` as a first-party in-ROM system asset
- [x] 1.2 Update internal asset paths to reference root-level canonical system styles and scripts (`/daisyui.css`, `/style.css`, `/themes.css`, `/agentla-bridge.js`)
- [x] 1.3 Add canonical top-rail navigation to `webui/servo.html` matching workbench layout (Home, Scope, Terminal, Files, Motor, Servo)

## 2. Firmware Route Registration

- [x] 2.1 Register `/servo` route in `firmware-rs/src/web.rs` backed by `SystemWebFile::new("servo.html", "text/html")`
- [x] 2.2 Register 301 redirect alias for `/servomaster` in `firmware-rs/src/web.rs` pointing to `/servo`
- [x] 2.3 Verify `firmware-rs` compiles cleanly with new route mappings

## 3. Navigation Rail, Dock & Deep Linking Integration

- [x] 3.1 Add Servo navigation link to `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, and `webui/barefoc.html`
- [x] 3.2 Formalize Servo application item in floating dock and application drawer in `webui/agentla-bridge.js` and `webui/app.js`
- [x] 3.3 Ensure active state styling uses surface highlights with zero CSS dot indicators
- [x] 3.4 Wire deep linking and hash modes (`/servo` and `#/servo`) in `webui/app.js`

## 4. Packaging, Live Deployment & Verification

- [x] 4.1 Update and run `tools/pack_webui.py` to package `servo.html` into `build/webui.img` and `build/webui.tar`
- [x] 4.2 Validate asset size within `SYSTEM_FS` partition headroom
- [x] 4.3 Verify that navigating to `/servo` serves Servo Master directly from `SYSTEM_FS`
