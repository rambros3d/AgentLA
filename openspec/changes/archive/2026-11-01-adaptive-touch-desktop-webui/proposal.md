## Why

The current AgentLA Web UI relies on a desktop window manager (`WinBox`) with floating, draggable windows. While functional with a mouse, floating windows introduce severe friction on touchscreens (tablets, iPads, phones, and bench monitors): tiny 2px–3px resize handles, accidental window dragging during list scrolling, window overlap obscuring active tools, and interference with logic analyzer waveform pinch-to-zoom gestures.

Replacing WinBox with a modern, touch-first and desktop-optimized workspace hub provides an intuitive, clutter-free bench experience. The interface combines a thumb-friendly bottom navigation bar on mobile/tablets with a sleek left rail on desktop, introduces a Unified Dashboard landing page with interactive pin status, provides a dual-slot split view for simultaneous side-by-side debugging, and adds full light and deep dark theme support.

## What Changes

- **Retire WinBox Window Manager**: Remove `winbox.js` and `winbox.min.css` from the embedded web assets, reclaiming valuable MCU flash memory and removing third-party DOM dependencies.
- **Adaptive Workspace Shell**: Implement a responsive full-screen shell that adapts seamlessly between touch devices and desktop:
  - Mobile & Portrait Tablets (<900px): Persistent Bottom Navigation Bar with WCAG-compliant $\ge 48\text{px}$ touch targets.
  - Desktop & Landscape Tablets (≥900px): Compact Left Navigation Rail maximizing horizontal trace width.
  - Uses dynamic viewport units (`100dvh`, `100dvw`) to prevent mobile address bar reflows.
- **Unified Dashboard Overview**: Establish a comprehensive default landing view featuring live probe health telemetry (CPU, voltage, frequency, USB status), an interactive 30-pin domain map (`GPIO 0–29`) showing real-time pin claims, and 1-tap instrument quick-launch tiles.
- **Dual-Slot Split View**: Add a dockable split workspace supporting side-by-side (landscape) or stacked (portrait) operation of two simultaneous instruments (e.g., Logic Analyzer + I3C Bus Controller), complete with 1-tap presets (Bus Debug, Target Bring-up) and custom tool assignment.
- **Drawer / Bottom Sheet Configuration**: Replace modal popups with slide-up bottom sheets on mobile and slide-out right drawers on desktop with native outside tap and swipe dismissal.
- **Multi-Touch Waveform Gestures**: Implement native `touch-action: none` gesture controls for the Logic Analyzer canvas (pinch-to-zoom, two-finger pan, double-tap zoom-to-fit).
- **Dual Theme Engine (Deep Dark & Light)**: Add native CSS custom-property theme switching between AgentLA's deep dark slate (`#08111d`) and a clean high-contrast Light Theme for brightly lit lab benches.

## Capabilities

### New Capabilities
- `adaptive-workspace-hub`: Responsive full-screen instrument workspace with adaptive navigation (Bottom Bar on touch viewports, Left Rail on desktop), a Unified Dashboard landing view, and slide-up/slide-out configuration inspectors.
- `dual-slot-split-view`: Dockable two-pane workspace manager enabling simultaneous side-by-side or stacked instrument monitoring with quick presets.
- `bench-theme-engine`: Dual-mode visual theming providing Deep Dark and high-contrast Light themes with persistent client-side state.

### Modified Capabilities
<!-- None: existing hardware control, MCP, and network bridge requirements remain unchanged -->

## Impact

- **Frontend Assets**: Complete redesign of `firmware/html/index.html`, `firmware/html/style.css`, and `firmware/html/app.js`. Deletion of `firmware/html/winbox.js` and `firmware/html/winbox.min.css`.
- **Build System**: Update `CMakeLists.txt` file list for `pico_set_lwip_httpd_content` to remove WinBox files.
- **Firmware Flash Memory**: Decreases embedded flash footprint by ~20 KB.
- **Runtime APIs**: Preserves all existing REST and JSON-RPC `/mcp` endpoints and SSE event streams.
