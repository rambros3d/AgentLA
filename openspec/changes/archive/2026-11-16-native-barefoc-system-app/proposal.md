## Why

BareFOC Studio is currently installed as a user-space application inside the mutable LittleFS filesystem. While functional, it requires manual package installation, consumes user-space storage capacity, and does not survive filesystem formatting or recovery operations. 

Promoting BareFOC Studio to a native in-ROM / `SYSTEM_FS` system application establishes it as a permanent, first-class workbench instrument alongside Scope, Terminal, and Files. This enables AgentLA to connect, configure, program, and debug external BareFOC motor targets out of the box with zero user installation steps.

## What Changes

- Add `webui/barefoc.html`: Host the full BareFOC Studio interface directly in the pre-compressed `SYSTEM_FS` asset directory.
- Update `firmware-rs/src/web.rs`:
  - Route `/motor` to `SystemWebFile::new("barefoc.html", "text/html")`.
  - Add redirect aliases for `/foc` and `/barefoc` pointing to `/motor`.
- Integrate into AgentLA OS Navigation & Shell:
  - Add **Motor** instrument to the top navigation rail across `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, and `webui/barefoc.html`.
  - Add BareFOC Studio icon and launcher tile in `webui/agentla-bridge.js` dock and the desktop application drawer.
- Update `tools/pack_webui.py` to bundle `barefoc.html` into `SYSTEM_FS` LittleFS image and update tarballs.
- Provide deep workbench coupling:
  - Direct target power rail control and short-circuit trip telemetry.
  - Software-switchable 120-ohm CAN termination.
  - Quick-link cross-launch with Logic Analyzer (`/scope`) with pre-labeled motor PWM and encoder channels.

## Capabilities

### New Capabilities
- `native-barefoc-system-app`: Native in-ROM integration of BareFOC Studio as an out-of-the-box system workbench instrument routed at `/motor` and embedded in the OS navigation and dock.

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `webui/barefoc.html`: Added as a permanent system asset.
- `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, `webui/agentla-bridge.js`: Top navigation and dock updated to include Motor instrument.
- `firmware-rs/src/web.rs`: Routing table updated with `/motor`, `/foc`, and `/barefoc`.
- `SYSTEM_FS`: Packaged image size increases by ~22 KB gzipped (well within the 512 KB budget).
- User LittleFS partition: Frees ~110 KB of user flash storage.
