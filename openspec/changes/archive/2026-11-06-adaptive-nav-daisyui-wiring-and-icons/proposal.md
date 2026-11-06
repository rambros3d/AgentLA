## Why

A JavaScript reference crash (`escapeHtml is not defined`) during page load breaks event listener attachment across the entire WebUI, leaving hardware profile buttons, the pin configuration modal, and MSO engine controls unresponsive. Additionally, hardcoded Slate dark palette colors in the profile dropdown and modals bypass the DaisyUI theme engine (causing severe contrast and visibility bugs in light mode), navigation icons are visually inconsistent, and the compact navigation bar forces rigid fixed-width tabs rather than dynamically scaling with content and available viewport width.

## What Changes

- **Fix WebUI Initialization Crash**: Define `escapeHtml` utility and robustly guard DOM initialization so all modules (MSO splitter, logic analyzer, scope, profile manager, pin config modal) mount reliably.
- **Wire Profile Manager MCP Calls**: Update `callMcp` profile invocation contracts (`callMcp('cmd', 'profile_list')`, `profile_safe`, `profile_apply`, `profile_save`) and add simulated mock profile endpoints in `web/demo/mock-bridge.js`.
- **Wire Scope Probe Pins Bar**: Ensure GP10–GP29 probe pin buttons cleanly open the interactive pin configuration modal for label editing, 18-color palette assignment, and trigger thresholds.
- **Unify DaisyUI Theme Tokens**: Replace all hardcoded Slate hex codes (`#0f172a`, `#1e293b`, `#020617`, `#334155`, `#38bdf8`) with standard DaisyUI design tokens (`--color-base-100`, `--color-base-200`, `--color-base-300`, `--color-primary`, `--border-color`), unifying accents to Emerald Green (`--color-primary`) matching the DaisyUI Forest palette.
- **Dynamic Content-Aware Bottom Navigation Bar**:
  - In compact/narrow viewports, tabs dynamically scale content-aware: inactive tabs collapse to snug icon-only touch targets, while the active tab expands to house both icon and text label in an animated pill.
  - When viewport space is available (wider screens before the vertical sidebar breakpoint), automatically display both icon and text label across all tabs simultaneously.
- **Solar Duotone Navigation Icons**: Standardize primary navigation icons to Solar Duotone SVG:
  - Scope: `graph`
  - Adapters: `link-circle`
  - Debugger: `magnifier-bug`
  - Pin Matrix: `cpu` / `pinout`
  - Console: `programming`

## Capabilities

### Modified Capabilities
- `adaptive-workspace-hub`: Update navigation shell requirements for dynamic content-aware pill scaling in compact viewports, dual icon+text display when space permits, and Solar Duotone icons (`graph`, `link-circle`, `magnifier-bug`, `programming`).
- `bench-theme-engine`: Unify theme accent variables to Emerald Green (`--color-primary`) and mandate zero hardcoded palette colors in profile dropdowns, modals, and control surfaces.
- `probe-pin-configuration`: Require reliable click-to-configure modal interactions on observation probe pins GP10–GP29 with HTML-escaped labeling.
- `hardware-profile-manager`: Require valid MCP JSON-RPC protocol compliance (`method=cmd&name=profile_*`) and browser simulator mock profile support.

## Impact

- `firmware/html/app.js`: Adds `escapeHtml`, corrects `callMcp` profile dispatch, ensures robust module initialization.
- `firmware/html/index.html`: Updates SVG icon markup for desktop navigation rail and mobile dock.
- `firmware/html/style.css`: Implements dynamic content-aware flex pill navigation, removes hardcoded Slate colors, binds profile dropdown to DaisyUI tokens.
- `web/demo/mock-bridge.js`: Adds mock hardware profile persistence and list simulation for demo mode.
