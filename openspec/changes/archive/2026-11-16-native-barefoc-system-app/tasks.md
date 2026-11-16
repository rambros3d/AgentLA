## 1. Web Asset Migration & Adaptation

- [x] 1.1 Copy and adapt `apps/barefoc/index.html` to `webui/barefoc.html` as a first-party system asset
- [x] 1.2 Update internal asset paths to reference root-level canonical system styles and scripts (`/daisyui.css`, `/style.css`, `/themes.css`, `/agentla-bridge.js`)
- [x] 1.3 Add top-rail navigation to `webui/barefoc.html` matching canonical workbench layout (Home, Scope, Terminal, Files, Motor)

## 2. Firmware Route Registration

- [x] 2.1 Register `/motor` route in `firmware-rs/src/web.rs` backed by `SystemWebFile::new("barefoc.html", "text/html")`
- [x] 2.2 Register 301 redirect aliases for `/foc` and `/barefoc` in `firmware-rs/src/web.rs` pointing to `/motor`
- [x] 2.3 Verify `firmware-rs` builds cleanly with new route mappings

## 3. Navigation Rail & Dock Integration

- [x] 3.1 Add Motor navigation link to `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, and `webui/files.html`
- [x] 3.2 Add Motor application launcher item to dock in `webui/agentla-bridge.js`
- [x] 3.3 Ensure active state styling uses surface highlights with zero CSS dot indicators

## 4. Packaging, Live Deployment & Verification

- [x] 4.1 Update and run `tools/pack_webui.py` to package `barefoc.html` into `build/webui.img` and `build/webui.tar`
- [x] 4.2 Stream `webui.tar` to `POST http://10.10.10.10/api/v1/system/webui/update` on live AgentLA hardware
- [x] 4.3 Verify that navigating to `http://10.10.10.10/motor` serves BareFOC Studio directly from `SYSTEM_FS`
