## Context

The AgentLA WebUI provides touch-and-desktop instrument control across RP2350 hardware. In recent refactorings, an undeclared `escapeHtml` function caused an unhandled runtime error on page load, breaking downstream DOM event binding. Simultaneously, the hardware profile dropdown was styled using raw Tailwind/Slate hex colors rather than DaisyUI tokens, causing dark blocks in light mode, and navigation tabs in compact mode were locked to static `flex: 1` widths without content-aware scaling.

## Goals / Non-Goals

**Goals:**
- Guarantee bulletproof WebUI initialization with standard HTML escaping and error-guarded initialization loops.
- Standardize all UI controls, modals, and dropdowns to DaisyUI v5 design tokens, unifying active highlights to Emerald Green (`--color-primary`).
- Implement dynamic content-aware navigation tabs:
  - Narrow/compact mode: Inactive tabs collapse to compact icon touch targets; active tab expands into an illuminated pill with icon + text label.
  - Intermediate/roomy mode: When viewport width permits (`min-width: 580px` up to desktop breakpoint), automatically display both icon and text label on all tabs simultaneously.
- Standardize primary navigation icons to Solar Duotone SVG (`graph`, `link-circle`, `magnifier-bug`, `cpu`/`pinout`, `programming`).
- Fix profile manager MCP command routing across both C firmware (`name=profile_*`) and browser simulator (`mock-bridge.js`).

**Non-Goals:**
- Reworking the underlying RP2350 flash layout or profile binary serialization format.
- Altering the digital logic analyzer capture engine PIO assembly.

## Decisions

### 1. Dynamic Content-Aware Navigation Tabs (CSS Flexbox & Media Queries)
- **Rationale**: Using pure CSS flexbox with `flex: 0 0 auto` and smooth transitions avoids JavaScript layout recalculations and jank.
- **Narrow Viewports (`< 580px`)**:
  - Inactive tabs: `min-width: 48px; padding: 4px 10px;` with `.btn-label-desktop { max-width: 0; opacity: 0; overflow: hidden; }`.
  - Active tab: `flex: 0 0 auto; background: color-mix(in oklch, var(--color-primary) 18%, transparent);` with `.btn-label-desktop { max-width: 120px; opacity: 1; }`.
- **Roomy Viewports (`580px` to `899px`)**:
  - All tabs display both icon and label inline with `.btn-label-desktop { max-width: 120px; opacity: 1; display: inline-block; }`.
  - Centered dock alignment with `justify-content: center; gap: 8px;`.

### 2. Accent Color Unification (Emerald Green `--color-primary`)
- **Rationale**: Aligns strictly with the DaisyUI Forest palette.
- Replaces `--accent-cyan` in active tabs, focus rings, and selection badges with `--color-primary`.
- Cyan (`--color-info`) is retained strictly for informational readouts (e.g. detected target summaries, firmware versions).

### 3. Missing Utility and MCP Routing Fix
- Add `escapeHtml(str)` utility at the top of `app.js`.
- Normalize `callMcp(method, name, args)` to ensure `name` is passed explicitly for profile actions (`callMcp('cmd', 'profile_list')`, `callMcp('cmd', 'profile_safe')`, `callMcp('cmd', 'profile_apply', slot)`).
- Implement simulated profile slots (Slot 0 Safe State + Slot 1 Default Lab Bench) in `mock-bridge.js` so demo mode operates seamlessly.

### 4. Solar Duotone Vector Paths
- Each icon uses two SVG elements:
  - Backdrop path: `opacity="0.5"` `fill="currentColor"` (housing, grid axes, chassis, screen).
  - Feature path: `fill="currentColor"` or `stroke="currentColor"` (waveform line, chain link, magnifying lens, code brackets).

## Risks / Trade-offs

- *[Risk]* Wide labels might overflow on extremely narrow viewports ($< 320\text{px}$).  
  → *Mitigation*: Set `max-width: 90px` on label text with `text-overflow: ellipsis; white-space: nowrap;` and use `clamp()` on container padding.
- *[Risk]* Caching issues in browser with updated CSS tokens.  
  → *Mitigation*: Automated cache-busting version query parameters in CMake asset staging pipeline.
