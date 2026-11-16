## Why

The existing floating glass dock uses `position: fixed` centered at the bottom of the viewport (`bottom: 0; left: 50%; transform: translateX(-50%)`), which floats over and obscures bottom-row controls, graticules, decoded packet tables, and terminal outputs. To prevent obscuration, disparate views historically injected artificial bottom padding (`padding-bottom: 72px` to `84px`), wasting valuable vertical height on desktop displays (where vertical space is constrained) and creating awkward dead space across workspaces.

Replacing the floating dock with an in-flow, adaptive navigation rail—a compact 48px vertical sidebar on wide screens (>= 900px) below the top bar, and a full-width 48px bottom toolbar on narrow screens (< 900px)—eliminates all content overlap, reclaims 100% vertical display height for instrumentation canvases, and delivers a superior, professional IDE-grade UX.

## What Changes

- **Convert Floating Dock to In-Flow Adaptive Rail**: Replace `.floating-glass-dock` with an in-flow `<nav id="app-nav-rail" class="app-nav-rail">` element integrated directly into the flexbox layout shell.
- **Desktop Sidebar Mode (>= 900px)**: Render the rail as an icon-only 48px vertical sidebar on the left side, positioned directly below the 100% full-width top bar (`border-r border-base-300`).
- **Mobile Bottom Bar Mode (< 900px)**: Render the rail as a full-width 48px horizontal bottom toolbar anchored flush with the bottom edge (`border-t border-base-300`).
- **Eliminate Dead-Space Padding Hacks**: Prune obsolete `padding-bottom: calc(6.5rem ...)` and `padding-bottom: 72px/84px` across all views (`.launcher-workspace`, `.main-workspace`, `.adapters-container`, `.scope-container`), allowing `#main-workspace` to occupy full flex height without clipping.
- **Relocate Telemetry & Storage Popovers**:
  - Desktop: Open Hardware Telemetry HUD and Device Storage popovers to the right of the sidebar (`left: calc(100% + 8px)`).
  - Mobile: Open popovers upwards above the bottom toolbar (`bottom: calc(100% + 8px)`).
- **Synchronize Tooltip Placement**: Tooltips fly out to the right in desktop sidebar mode, and are suppressed or touch-adapted in mobile bottom bar mode.
- **Synchronize Across All Surfaces**: Update [`webui/index.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/index.html), [`webui/lab.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/lab.html), [`webui/files.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/files.html), [`webui/agentla-bridge.js`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/agentla-bridge.js), and [`webui/style.css`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/style.css).

## Capabilities

### New Capabilities
- `adaptive-navigation-rail`: In-flow responsive navigation rail establishing a 48px left vertical sidebar below the top bar on desktop viewports (>= 900px) and a 48px full-width horizontal bottom bar on mobile viewports (< 900px).

### Modified Capabilities
- `adaptive-workspace-hub`: Updates navigation shell layout contract from floating centered overlay dock to in-flow adaptive rail, removing dead-space bottom padding from workspace containers.

## Impact

- **WebUI Markup**: [`webui/index.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/index.html), [`webui/lab.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/lab.html), [`webui/files.html`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/files.html).
- **WebUI Styling**: [`webui/style.css`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/style.css) (dock layout rules, popover positioning, padding cleanup).
- **JavaScript Bridges**: [`webui/app.js`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/app.js), [`webui/agentla-bridge.js`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/webui/agentla-bridge.js).
- **LittleFS Image**: Packaged via [`tools/pack_webui.py`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/tools/pack_webui.py).
