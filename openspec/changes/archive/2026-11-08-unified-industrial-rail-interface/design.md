## Context

AgentLA is an embedded hardware diagnostic instrument, logic analyzer, and flash programmer. The WebUI currently supports two divergent aesthetic styles: the consumer-oriented Bento persona (`data-style="bento"`) with large rounded corners (1.25rem), drop shadows, and soft glass cards, and the Industrial persona (`data-style="industrial"`) featuring dot-matrix headings, monospaced metadata, and OLED dark surfaces. 

Maintaining both styles creates redundant CSS rules, dual rendering branches, and configuration friction in the settings modal. Furthermore, the floating dock currently uses a 24px rounded pill shape that contradicts the sharp industrial identity, and the NDot dot-matrix font lacks lowercase glyphs, preventing mixed-case display titles.

This design unifies the entire WebUI onto a single, uncompromising Industrial aesthetic built directly on DaisyUI 5, introduces an authentic Industrial Light mode ("Technical Alabaster"), transforms the dock into an Industrial Chamfered Rail, adds dockable workspace widgets for live telemetry and storage, and expands the NDot font to full ASCII.

## Goals / Non-Goals

**Goals:**
- Permanently retire the Bento persona and establish the Industrial style as the sole design system across `index.html`, `lab.html`, `terminal.html`, and `files.html`.
- Maintain DaisyUI 5 as the core component framework, utilizing semantic tokens (`--color-base-100`, `--color-primary`, `--radius-box`, `--border`) without adding third-party UI libraries.
- Support both **Industrial Dark** (OLED `#000000`, `#0a0a0a` surfaces, `#D71921` accent) and **Industrial Light** (Alabaster `#ffffff`, `#f5f5f5` surfaces, `#0f0f0f` text, `#D71921` accent, inverted dot-matrix background).
- Expand `tools/generate_ndot_font.py` with 5x7 circular dot bitmaps for lowercase letters `a-z`, generating a complete ASCII WOFF2 font (~2.2 KB).
- Implement a 3-tier hybrid typography system: NDot dot-matrix for headings/tiles, technical monospace (`ui-monospace, monospace`) for metadata/tables/badges/buttons, and clean sans-serif for long paragraphs.
- Restyle the floating bottom dock as an Industrial Chamfered Rail (sharp 0.5rem squircle box container with hairline dividers and Signal Red active indicator pips).
- Implement dockable canvas widgets allowing the live Telemetry HUD and Flash Storage monitors to be pinned directly to the workspace canvas while retaining dock popover toggles.
- Redesign `files.html` into the Industrial Storage Console.

**Non-Goals:**
- Replacing DaisyUI or Tailwind CSS with an external framework.
- Modifying firmware REST API endpoints (`/api/v1/system/telemetry`, `/api/v1/fs/*`).
- Removing the bottom dock or altering its tool launch shortcuts.

## Decisions

### 1. DaisyUI Theme Architecture for Industrial Dark and Light
- **Rationale**: DaisyUI's CSS custom property architecture allows instant runtime theme toggling without DOM reloads. We configure `nothing` (Dark) and `industrial-light` (Light) with identical geometry (`--radius-box: 0.5rem; --border: 1px; --depth: 0;`), swapping only color tokens and inverted canvas dot-matrix gradients (`rgba(255, 255, 255, 0.12)` in dark vs `rgba(0, 0, 0, 0.12)` in light).
- **Alternatives Considered**: Retaining Emerald or CaramelLatte as light themes was rejected because their 1rem rounded geometry and green/brown accents clash with the sharp Industrial design language.

### 2. 5x7 Lowercase Circular Dot-Matrix Font Generation
- **Rationale**: The existing `ndot57.woff2` only contains `A-Z`, `0-9`, and symbols, causing lowercase characters to either fail or fall back to system fonts. Generating lowercase `a-z` using 5x7 circular dot matrices via `fontTools` in `tools/generate_ndot_font.py` produces a self-hosted WOFF2 font of ~2.2 KB, well within our 512 KB `SYSTEM_FS` budget.
- **Alternatives Considered**: Forcing uppercase on all dot-matrix text via CSS `text-transform: uppercase` was rejected because mixed-case display titles and acronyms need visual nuance.

### 3. Chamfered Rail Dock with Signal Red Indicator
- **Rationale**: Changing the dock container from `border-radius: 24px` to `border-radius: 8px` (0.5rem) with a 1px border (`rgba(255, 255, 255, 0.12)` in dark, `#e5e5e5` in light) harmonizes the dock with the surrounding cards, squircle tiles, and popover panels. A Nothing Signal Red (`#D71921`) active dot provides high-contrast, authentic status indication.
- **Alternatives Considered**: A fixed full-width bottom taskbar was rejected because it consumes too much vertical screen real estate on mobile and tablet screens.

### 4. Dockable Canvas Widgets Architecture
- **Rationale**: Telemetry and Storage information serve two distinct user needs: quick momentary checks (popover) vs persistent continuous bench monitoring (pinned workspace widget). Adding a "Pin to Workspace" action button to the popover allows users to mount the live telemetry HUD card onto the main workspace canvas.
- **State Management**: Mount states are tracked in an array in `localStorage` under `agentla_pinned_widgets` (`['telemetry', 'storage']`). During page hydration, pinned widgets are rendered into a dedicated `#pinned-widgets-area` above the instrument grid.

## Risks / Trade-offs

- **[Risk] Increased font size impacting `SYSTEM_FS` partition headroom** → *Mitigation*: Adding lowercase `a-z` adds only 26 glyphs to the WOFF2 font, increasing file size by ~900 bytes (from 1.3 KB to ~2.2 KB). With 158 KB of headroom remaining in `SYSTEM_FS` (30.9% free), this has negligible impact.
- **[Risk] Light mode legibility of dot-matrix headings** → *Mitigation*: High-contrast jet black (`#0f0f0f`) on `#ffffff` ensures sharp, legible dot-matrix glyphs even at smaller header sizes.
- **[Risk] FOUC on pinned canvas widgets during initial load** → *Mitigation*: Inline synchronous script in `<head>` checks `localStorage.getItem('agentla_pinned_widgets')` and injects matching classes/DOM elements before first layout pass.
