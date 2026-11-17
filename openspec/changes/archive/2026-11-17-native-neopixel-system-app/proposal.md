## Why

NeoPixel Control is currently deployed as a user application in the mutable LittleFS filesystem (`USER_FS` at `apps/neopixel/`). While functional, it requires manual package installation, consumes mutable storage capacity, and is erased during filesystem formatting, partition recovery, or factory reset operations. 

Simultaneously, the Model Context Protocol (MCP) tool surface currently provides only a static placeholder (`ws2812_transmit`) with hardcoded RGB pixels and an empty schema, preventing autonomous AI agents from configuring or driving addressable LEDs.

Promoting NeoPixel Control to a native in-ROM / `SYSTEM_FS` application and equipping the MCP server with a full addressable LED toolset establishes NeoPixel Control as a permanent workbench instrument alongside Scope, Terminal, Files, Motor, and Servo, while giving AI agents full programmatic control over LED strips and matrices.

## What Changes

- Add `webui/neopixel.html`: Promote and host NeoPixel Control directly in the pre-compressed `SYSTEM_FS` ROM asset directory.
- Update `firmware-rs/src/web.rs`:
  - Route `/neopixel` to `SystemWebFile::new("neopixel.html", "text/html")`.
  - Add redirect alias `/led` pointing to `/neopixel`.
- Integrate into AgentLA OS Navigation & Shell:
  - Add **NeoPixel** instrument to the top navigation header across all canonical system pages (`index.html`, `lab.html`, `terminal.html`, `files.html`, `barefoc.html`, `servo.html`, `neopixel.html`).
  - Formalize NeoPixel launcher item in floating dock and application drawer in `webui/agentla-bridge.js` and `webui/app.js` using the Solar icon **Bold Duotone Filters** (`ref/solar-icons/packages/core/svgs/tools/BoldDuotone/filters.svg`).
  - Configure deep linking for `/neopixel` and `#/neopixel`.
- Update `tools/pack_webui.py`:
  - Add `neopixel` to `SYSTEM_APPS` to bundle `neopixel.html` into `SYSTEM_FS` LittleFS image and update tarballs.
- Expand MCP Tool Surface:
  - Add tools in `crates/agentla-mcp/src/tools.rs`:
    - `neopixel_fill`: Fill N pixels with a uniform color.
    - `neopixel_set`: Update a single pixel at a specified index.
    - `neopixel_clear`: Blank / power-off all pixels.
    - `neopixel_pattern`: Drive procedural patterns (`rainbow`, `cylon`, `fire`, `meteor`, `pulse`, `plasma`).
    - `neopixel_analyze`: Measure WS2812 signal timing margins.
    - `neopixel_transmit`: Transmit an arbitrary array of colors (upgrades `ws2812_transmit`).
  - Implement JSON-RPC tool dispatch in `firmware-rs/src/mcp.rs`.
  - Update `webui/mcp.json` tool capability count from 40 to 45.

## Capabilities

### New Capabilities
- `native-neopixel-system-app`: Native in-ROM integration of NeoPixel Control as an out-of-the-box system workbench instrument routed at `/neopixel` (with `/led` alias) and embedded in the OS navigation header and floating dock with Solar bold duotone filters icon.
- `mcp-neopixel-controls`: Direct MCP tool interface for addressable LED actuation (fill, set, clear, pattern, timing analysis, and raw transmission).

### Modified Capabilities
<!-- No requirement changes to existing capabilities -->

## Impact

- `webui/neopixel.html`: Added as a permanent system asset in ROM.
- `webui/agentla-bridge.js` & `webui/app.js`: Solar bold duotone filters SVG icon and dock button added.
- `firmware-rs/src/web.rs`: Routing table updated with `/neopixel` and `/led`.
- `crates/agentla-mcp` & `firmware-rs/src/mcp.rs`: 5 new/upgraded MCP tools added.
- `webui/mcp.json`: Tool count incremented to 45.
- `SYSTEM_FS`: Packaged image size increases by ~13.8 KB gzipped (well within the 512 KB budget and ~129.7 KB available margin).
- `USER_FS`: Frees mutable user flash storage previously occupied by user app files.
