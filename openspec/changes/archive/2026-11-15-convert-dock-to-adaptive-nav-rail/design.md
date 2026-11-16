## Context

The current WebUI features a floating, centered dock pill (`.floating-glass-dock`) styled after macOS with `position: fixed; bottom: 0; left: 50%; transform: translateX(-50%)`. While visually distinct, this floating element covers the bottom-center region of the workspace, obstructing critical observation elements such as logic analyzer trace graticules, serial terminal prompt lines, and decoded protocol packet drawers. 

To prevent content from being covered, several views implemented artificial padding (`padding-bottom: 72px` and `padding-bottom: calc(6.5rem + env(safe-area-inset-bottom, 0px))`). On desktop 1080p and laptop screens where vertical height is already scarce (approx. 900px usable browser viewport), this wasted vertical pixels and created dead whitespace shelves.

## Goals / Non-Goals

**Goals:**
- Transition the navigation controls from a floating fixed-position overlay into an in-flow `<nav id="app-nav-rail" class="app-nav-rail">` integrated into the body flexbox structure.
- Desktop viewports (>= 900px): Present an icon-only 48px vertical rail on the left edge below the top bar, maximizing vertical observation canvas height.
- Mobile viewports (< 900px): Present an in-flow 48px full-width horizontal bottom bar anchored directly to the bottom edge.
- Eliminate artificial bottom padding hacks (`padding-bottom: 72px/84px`) across all workspace views.
- Anchor Telemetry HUD and Device Storage popovers adjacent to their triggers (to the right on desktop, above on mobile).
- Ensure 100% adherence to project invariants: zero emojis, no CSS dot pips, full-bleed viewports, centered launcher (`margin-inline: auto`), and LittleFS flash budget preservation.

**Non-Goals:**
- Adding collapsible text label expansions in the sidebar (icon-only 48px rail confirmed).
- Changing top-bar functionality (Target Power Rail, Profile split-button, Theme toggle, and Settings remain in the top bar).
- Restructuring internal instrument logic or backend REST/MCP APIs.

## Decisions

### 1. In-Flow Flexbox Architecture vs. Fixed Overlay
- **Decision**: Place `<nav id="app-nav-rail">` and `<main id="main-workspace">` as direct flex siblings inside an `.app-shell` container below `<header id="top-bar">`.
- **Rationale**: In-flow layout inherently guarantees that workspace content never overlaps with navigation controls. The browser flexbox engine automatically calculates available width and height, eliminating manual padding compensation.
- **Alternatives considered**:
  - *Keep fixed overlay with CSS margin calculation*: Fragile and prone to zoom-level desynchronization.

### 2. Desktop Rail Positioning: Below Top Bar
- **Decision**: The top bar spans 100% width across the top of the viewport. The left navigation rail sits directly below the top bar on the left margin.
- **Rationale**: Preserves consistent header branding, power telemetry, and profile controls at the top of the screen while providing an activity bar directly accessible to the primary tools.
- **Alternatives considered**:
  - *Full-height rail extending to screen top*: Breaks top bar visual continuity and complicates responsive collapse.

### 3. Responsive Breakpoint & Orientation Switch (900px)
- **Decision**: Use `900px` (`@media (min-width: 900px)` vs `@media (max-width: 899.98px)`) as the breakpoint:
  - `>= 900px`: `.app-shell { flex-direction: row; }` -> `.app-nav-rail { width: 48px; height: 100%; flex-direction: column; border-right: 1px solid var(--border-color); }`
  - `< 900px`: `.app-shell { flex-direction: column-reverse; }` -> `.app-nav-rail { width: 100%; height: 48px; flex-direction: row; border-top: 1px solid var(--border-color); }`
- **Rationale**: Matches the existing desktop/tablet breakpoint used throughout AgentLA stylesheets.

### 4. Popover Docking Geometry
- **Decision**:
  - In desktop sidebar mode, popovers (`#telemetry-hud` and `#storage-glass-popover`) anchor with `left: calc(100% + 8px); bottom: 8px;`.
  - In mobile bottom bar mode, popovers anchor with `bottom: calc(100% + 8px); left: 50%; transform: translateX(-50%);`.
- **Rationale**: Ensures the popovers never occlude the navigation trigger icons themselves and remain within comfortable viewing bounds.

## Risks / Trade-offs

- **[Risk] Reduced horizontal space on desktop**: The 48px sidebar consumes 48px of width.
  - *Mitigation*: 48px is less than 2.5% of a standard 1920px screen and recovers 48px-84px of vertical height, which is far more valuable for oscilloscope and logic analyzer graticules.
- **[Risk] Legacy scripts querying `#floating-glass-dock`**: Embedded scripts or SDKs might query the old ID.
  - *Mitigation*: Retain `#floating-glass-dock` as an ID or alias alongside `#app-nav-rail` during markup migration.
