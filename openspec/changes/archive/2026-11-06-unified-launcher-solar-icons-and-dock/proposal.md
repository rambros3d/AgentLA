## Why

The AgentLA App Launcher (`/apps`) and Lab Workbench (`/lab`) currently present subtle visual and functional discrepancies: the launcher lacks the hardware profile switcher in its top bar, uses generic thin-stroke SVGs instead of authentic 2-tone Solar Icons, places storage capacity details in a page-flow card rather than an operating-system dock, and labels core instruments as "CORE INSTRUMENTS & FLAGSHIP". Aligning the launcher with the Lab workbench creates a seamless, cohesive embedded OS experience while ensuring third-party user applications can render their own custom graphics.

## What Changes

- **Unified Top Application Bar**: Replicate the exact Lab top bar in `/apps`, adding the Flash Hardware Profile dropdown selector (`#profile-dropdown-wrapper`) and maintaining display-only branding on the logo.
- **Solar Iconography for Core Apps**: Rename the core section from "CORE INSTRUMENTS & FLAGSHIP" to "Core Apps" and replace generic feather-style strokes with authentic 2-tone Solar Icons (Lab flagship bench, MSO Scope, Pinout Matrix, USB Adapters, Console).
- **Custom User-Installed App Icons**: Support inline SVGs (`iconSvg`) from `app.json`, standalone root icons (`icon.svg` / `icon.png` in `/apps/{app-id}/`), and provide a polished 2-tone Solar app box fallback.
- **Fixed OS Bottom Dock for Storage**: Move flash storage partition details (ROM, LittleFS, Free flash) into a fixed DaisyUI bottom `.dock` with live byte metrics and multi-segment color visualizers, with appropriate viewport bottom padding.

## Capabilities

### New Capabilities
<!-- No brand new standalone capabilities required; builds upon unified OS launcher architecture -->

### Modified Capabilities
- `unified-os-app-launcher`: Rename core instruments section to "Core Apps", update top bar with Hardware Profile dropdown, introduce fixed bottom dock for flash partition telemetry, and implement multi-format user app icon resolution hierarchy.

## Impact

- `firmware/html/apps.html`: Top bar layout, "Core Apps" section markup, Solar SVG replacements, user app rendering script, and fixed bottom dock.
- `firmware/html/style.css`: Styling for the fixed OS bottom dock (`.dock`, storage indicators) and custom user app icons.
- `tools/gen_board_assets.py`: Staging and compiling updated launcher assets into C header flash arrays.
- `web/apps.html`: Synchronized copy for hosted static preview parity.
