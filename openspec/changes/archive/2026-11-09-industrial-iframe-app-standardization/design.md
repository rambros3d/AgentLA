## Context

AgentLA integrates a persistent iframe application shell (`persistent-iframe-app-shell`) that hosts both system applications (`terminal.html`, `files.html`) and user applications (`canbus`, `neopixel`, `servo`).

When these applications run inside child iframes:
1. The host shell injects `class="is-embedded"` onto `<html>`, activating the CSS rule `html.is-embedded #top-bar { display: none !important; }`.
2. The bottom of the viewport is overlayed by the 48px glass dock. Any action control pinned to the bottom (such as `#cmd-input` in `terminal.html` or the frame injector in `canbus`) becomes occluded or unclickable.
3. Applications that put primary controls (e.g. shader buttons in `neopixel` or partition switches in `files`) in the top navigation header lose access to them completely when embedded.
4. Legacy user applications use heterogeneous dark styles (`#0b0f19`) rather than conforming to the system industrial theme (`nothing` / `industrial-light` with 16px dot-grid canvas and NDot57 typography).

## Goals / Non-Goals

**Goals:**
- Provide a standardized, unified embedded and standalone layout for all 5 applications (`terminal`, `files`, `canbus`, `neopixel`, `servo`).
- Invert the Serial Terminal layout to place command entry and controls at the top, rendering stream logs reverse-chronologically without scroll fighting.
- Elevate the CAN frame injector into a prominent top Bento card.
- Ensure all app-specific controls (Start Shaders, Blackout, Calibrate, Partitions) remain accessible via dedicated in-app sub-toolbars positioned beneath `#top-bar`.
- Prevent dock occlusion across all scrollable viewports using a strict 72px bottom clearance.
- Enforce the Industrial Design System and zero emojis across all 5 applications.

**Non-Goals:**
- Modifying backend firmware protocol engines (`firmware-rs`, `agentla-bridges`, `agentla-instruments`).
- Altering the host launcher dock architecture or iframe embedding contract (`agentla-bridge.js`).

## Decisions

### 1. In-App Sub-Toolbars vs Top-Bar Extension
- **Decision**: Place application-specific controls in a dedicated in-app sub-toolbar immediately below `#top-bar`.
- **Rationale**: When standalone, users see both the top-bar (branding and system power) and the app sub-toolbar. When embedded, the host hides `#top-bar`, leaving the app sub-toolbar intact at the very top of the iframe without duplicate headers.
- **Alternatives Considered**: Injecting controls into host top-bar via `postMessage`. Rejected because it introduces cross-frame fragility and fails when apps run standalone.

### 2. Inverted Serial Terminal Architecture
- **Decision**: Anchor `#cmd-input` and `#btn-send` directly beneath the baud/pin sub-toolbar at the top of the viewport. Render new log messages at the top of `#terminal-output` using `prependChild` / `insertAdjacentHTML('afterbegin')`.
- **Rationale**: Keeps the active interaction zone away from the bottom dock. Users immediately see prompt responses right below their input line without jumping to the bottom.
- **Alternatives Considered**: Bottom input with `72px` margin. Rejected because users still had to look back and forth across a tall screen and risk dock overlap.

### 3. CAN Bus Frame Injector Bento Placement
- **Decision**: Fix the TX Frame Injector as a top Bento card directly below the baud rate and filter toolbar. Place the packet log table below it.
- **Rationale**: Frame injection is a primary testing action alongside filtering; placing it at the top keeps it constantly accessible without scrolling past hundreds of received frames.

### 4. 72px Bottom Clearance Standard
- **Decision**: Add `padding-bottom: 72px` (or `pb-[72px]`) to the main scroll containers of all embedded views (`#terminal-container`, `#file-canvas`, `#can-packet-table`, `#neopixel-canvas`, `#servo-canvas`).
- **Rationale**: The host dock is 48px high with an additional floating buffer. A 72px clearance ensures the last row of data or buttons can scroll completely above the dock.

### 5. Industrial Design System Integration
- **Decision**: Standardize HTML attributes to `<html lang="en" data-theme="nothing" data-style="industrial">`.
- **Rationale**: Synchronizes DaisyUI CSS variables with `--font-display: 'NDot57'`, `--font-mono-tech`, black background (`#000000`), primary red highlight (`#D71921`), and technical dot-grid canvas.

## Risks / Trade-offs

- **[Risk] Reverse chronological log confusion**: Users accustomed to traditional bottom-scrolling terminals might be momentarily surprised by top-anchored logs.
  - *Mitigation*: The top-anchored input bar with a distinct `>` prompt visually anchors the flow, and an optional direction indicator badge ("STREAM: NEWEST TOP") clarifies the ordering.
- **[Risk] LittleFS flash size budget**: Additional CSS or HTML bloat could exceed the 512KB LittleFS partition limit.
  - *Mitigation*: Changes reuse existing `daisyui.css` utility classes and shared `style.css`/`themes.css` without adding new third-party dependencies or raster assets.
