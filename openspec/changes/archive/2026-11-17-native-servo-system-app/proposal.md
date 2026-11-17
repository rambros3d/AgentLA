## Why

Servo Master is currently deployed as a user-space application inside the mutable LittleFS filesystem (`USER_FS`). While functional, it requires manual package installation, consumes mutable storage capacity, and is erased during filesystem formatting, partition recovery, or factory reset operations.

Meanwhile, the RP2350 firmware kernel already implements first-class native driver support for RC PWM servos in the pin arbiter (`pin_owner::Driver::Servo`) and provides dedicated `/api/v1/servo/*` REST endpoints. Promoting Servo Master to a native in-ROM / `SYSTEM_FS` system application resolves this architectural asymmetry, establishing it as a permanent, first-class workbench instrument alongside Scope, Terminal, Files, and Motor. This guarantees out-of-the-box multi-channel PWM servo control, continuous rotation actuation, and keyframe pose sequencing with zero user installation steps.

## What Changes

- Add `webui/servo.html`: Promote and host the Servo Master workbench interface directly in the pre-compressed `SYSTEM_FS` ROM asset directory.
- Update `firmware-rs/src/web.rs`:
  - Route `/servo` to `SystemWebFile::new("servo.html", "text/html")`.
  - Add redirect alias `/servomaster` pointing to `/servo`.
- Integrate into AgentLA OS Navigation & Shell:
  - Add **Servo** instrument to the top navigation header across all canonical system pages (`webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, `webui/barefoc.html`, `webui/servo.html`).
  - Formalize Servo Master icon and launcher tile in `webui/agentla-bridge.js` dock and `webui/app.js` pinned apps.
- Update `tools/pack_webui.py` to bundle `servo.html` into the `SYSTEM_FS` LittleFS image and update tarballs.
- Provide deep workbench safety coupling:
  - Direct integration with Emergency Safe State (`Shift+Esc`) to immediately detach all channels and release claimed pins.
  - Live target rail voltage and current telemetry monitoring from `/api/v1/system/telemetry` to detect servo stalls and brownout conditions.

## Capabilities

### New Capabilities
- `native-servo-system-app`: Native in-ROM integration of Servo Master as an out-of-the-box system workbench instrument routed at `/servo` and embedded in the OS navigation header and floating dock.

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `webui/servo.html`: Added as a permanent system asset in ROM.
- `webui/index.html`, `webui/lab.html`, `webui/terminal.html`, `webui/files.html`, `webui/barefoc.html`, `webui/agentla-bridge.js`: Top navigation and dock updated to include Servo instrument.
- `firmware-rs/src/web.rs`: Routing table updated with `/servo` and `/servomaster`.
- `SYSTEM_FS`: Packaged image size increases by ~8.2 KB gzipped (well within the 512 KB budget and ~138 KB available margin).
- `USER_FS`: Frees mutable user flash storage previously occupied by the user app files.
