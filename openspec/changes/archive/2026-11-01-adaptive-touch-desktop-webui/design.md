## Context

AgentLA serves an embedded web application over USB RNDIS/NCM via lwIP HTTPD. Web assets (`firmware/html/`) are compiled into RP2350 flash using the Pico SDK `makefsdata.py` script. The existing UI utilizes a desktop window manager library (`WinBox`) which creates free-floating, overlapping draggable windows.

While WinBox works reasonably with a mouse on a large desktop monitor, it creates severe usability problems on touchscreens (tablets, phones, and bench monitors):
- Resizing requires precision dragging of a 2px–3px edge handle.
- Dragging a window frequently conflicts with scrolling internal data tables or logs.
- Canvas pinch-to-zoom for the logic analyzer waveform is intercepted or clipped by window boundaries.
- On smaller screens, floating windows obscure each other and top navigation bars.

We need a unified, touch-first and desktop-friendly architecture that provides optimal ergonomics across all form factors without sacrificing power-user capabilities like simultaneous multi-instrument monitoring.

## Goals / Non-Goals

**Goals:**
- Provide a responsive shell that adapts seamlessly between touch devices and desktop:
  - Viewports < 900px (Mobile & Portrait Tablets): Bottom Navigation Bar with WCAG-compliant $\ge 48\text{px}$ touch targets.
  - Viewports $\ge 900\text{px}$ (Desktop & Landscape Tablets): Left Navigation Rail maximizing horizontal trace width.
- Establish a Unified Dashboard landing view featuring probe health metrics, an interactive 30-pin domain allocation widget (`GPIO 0–29`), and 1-tap instrument tiles.
- Implement a Dual-Slot Split View (Option B) for simultaneous side-by-side or stacked instrument execution with 1-tap workflow presets (Bus Debug, Target Bring-Up) and manual slot assignment.
- Replace modal popups with slide-up Bottom Sheets (touch) and slide-out Right Drawers (desktop).
- Implement multi-touch gesture navigation (`touch-action: none`) on the waveform canvas for two-finger pinch-zoom, pan, and double-tap zoom reset.
- Implement native CSS Custom Property theme switching between Deep Dark (`#08111d`) and Light themes with `localStorage` persistence.
- Remove `winbox.js` and `winbox.min.css`, saving ~20 KB of flash memory.

**Non-Goals:**
- Heavy client-side JavaScript frameworks (React, Vue, Angular) which require Node build steps and increase flash footprint.
- Arbitrary floating window dragging.

## Decisions

### Decision 1: Adaptive Navigation Architecture (Bottom Nav vs. Left Rail)
- **Choice**: Use a Bottom Navigation Bar on viewports < 900px and a Left Icon Rail on viewports $\ge 900\text{px}$, combined with dynamic viewport units (`100dvh`, `100dvw`).
- **Rationale**: On mobile and portrait tablets, the bottom of the screen is within natural thumb reach. On desktop and landscape monitors, horizontal width is abundant while vertical height is precious for stacked logic analyzer channels. Dynamic viewport units (`dvh`) prevent mobile browser address bars from breaking the layout.
- **Alternatives Considered**:
  - *Keep top bar only with hamburger menu*: Requires awkward two-handed reaches to the top-left on tablets/phones.

### Decision 2: Dual-Slot Dockable Workspace
- **Choice**: Implement a 2-slot CSS Grid layout:
  - Single Mode: Active slot occupies 100% width and height.
  - Split Mode (Wide screen): `grid-template-columns: 1fr 1fr;` (side-by-side).
  - Split Mode (Narrow screen): `grid-template-rows: 1fr 1fr;` (stacked).
- **Rationale**: Completely eliminates window management overhead, window overlap, and edge resizing. Instruments always have deterministic, full-width or clean half-width viewports.
- **Alternatives Considered**:
  - *Arbitrary tiled grid (3+ slots)*: Too crowded on embedded tablet viewports. Two slots cover 99% of bench needs (e.g. Signal capture + Bus transmission).

```
┌────────────────────────────────────────────────────────────────────────┐
│                        Workspace Shell Layout                          │
├────────────────────────────────────────────────────────────────────────┤
│ [Top Bar: Logo | Health Badge | Mode Switch: ▤/▥ | Theme Toggle ☀️/🌙] │
├────────────┬─────────────────────────────┬─────────────────────────────┤
│ LEFT RAIL  │ SLOT 1                      │ SLOT 2 (Split View)         │
│ (Desktop)  │ [Tool Selector / Header]    │ [Tool Selector / Header]    │
│            │                             │                             │
│ 🏠 Hub     │ Active Instrument View      │ Active Instrument View      │
│ ⚡ Flasher  │                             │                             │
│ 📊 Logic   │ (e.g. Logic Analyzer        │ (e.g. I3C Controller /      │
│ 🔌 Adapter │  Waveform Canvas)           │  Serial Terminal)           │
│ 💻 Terminal│                             │                             │
├────────────┴─────────────────────────────┴─────────────────────────────┤
│ BOTTOM BAR (Touch/Mobile Viewport: 48px+ thumb targets)                │
└────────────────────────────────────────────────────────────────────────┘
```

### Decision 3: Inspector Drawer and Bottom Sheet Configuration
- **Choice**: Configuration panels (trigger settings, pin mappings, clock frequency) render in a slide-up Bottom Sheet on mobile and a slide-out Right Drawer on desktop.
- **Rationale**: Provides large, touch-accessible controls without occluding the entire screen or floating arbitrarily. Supports swipe-down to dismiss on touch devices.

### Decision 4: Waveform Multi-Touch Gestures
- **Choice**: Handle pointer events directly on the Logic Analyzer canvas with `touch-action: none`. Track primary and secondary pointer IDs to compute distance delta for pinch-to-zoom and center movement for horizontal panning.
- **Rationale**: Eliminates browser page-zoom interference and provides the responsive, fluid feel of native oscilloscope touchscreens.

### Decision 5: CSS Variable Theming (Deep Dark & Light)
- **Choice**: Define semantic tokens in `style.css` (`--bg-primary`, `--bg-surface`, `--bg-card`, `--text-primary`, `--border-color`, `--accent-cyan`, `--accent-blue`). Toggle `data-theme="dark"` or `data-theme="light"` on `document.documentElement`.
- **Rationale**: Zero runtime JavaScript overhead; styles apply instantly without flashes of unstyled content or layout recomputations.

## Risks / Trade-offs

- **[Risk] Canvas Resize when Toggling Split View** → **Mitigation**: Attach a `ResizeObserver` to each instrument slot container. When dimensions change, auto-resize `<canvas>` internal resolution (`width`, `height`) and redraw waveforms without distortion.
- **[Risk] 300ms Mobile Tap Delay** → **Mitigation**: Apply `touch-action: manipulation` across all buttons and interactive controls.
- **[Risk] Serial/SSE Stream Throughput during Split View** → **Mitigation**: Route SSE event listeners to active mounted tools only. Inactive background tools pause canvas rendering to conserve CPU cycles.
