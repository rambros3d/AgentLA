## Why

AgentLA is an authentic hardware diagnostic instrument, logic analyzer, and target programmer. While the current Bento consumer desktop provides an approachable squircle look, users in engineering and lab environments often prefer a high-precision, tactical, minimalist industrial aesthetic reminiscent of Nothing OS and Teenage Engineering.

Introducing a global dual-persona design system—with the **Nothing OS-inspired Industrial persona as the default** and **Bento as a selectable alternative** in System Settings—gives the probe an authentic lab identity without adding heavy UI framework bloat or exceeding bare-metal flash constraints.

## What Changes

- **Default Industrial Persona (`data-style="industrial"`)**: Out-of-the-box appearance featuring OLED black surfaces (`#000000`), sharp 1px technical outlines (`#1a1a1a`), an iconic Nothing Signal Red (`#D71921`) primary accent, and a subtle 16px technical dot-grid canvas.
- **Circular Dot-Matrix & Monospace Typography**:
  - Embedded subsetted `ndot57.woff2` (~5.5 KB raw / ~3.2 KB gzipped in `SYSTEM_FS`) for display headings, hero measurement digits, and system clocks.
  - High-tracking monospaced typographic stack for pin numbers, memory addresses, baud rates, and status tags (`[RO]`, `[RW]`, `[SAFE]`).
  - Standard clean sans/monospace body text to maintain optimal reading legibility.
- **Global Persona Switcher in System Settings**:
  - Adds an Interface Persona toggle (`[ ⚙️ Industrial (Default) ]` vs `[ 🍱 Bento ]`) in `#modal-settings`.
  - Persists preference in `localStorage.getItem('agentla-ui-style')` and restores it before DOM render across all web desktop pages (`index.html`, `lab.html`, `terminal.html`, `files.html`) to prevent theme flash.
- **Dedicated DaisyUI Theme (`themes/nothing.css`)**:
  - Compiled into `webui/themes.css` via `tools/compile_themes.py`, defining exact tokens for Signal Red, OLED blacks, and 0.5rem radius geometry.

## Capabilities

### New Capabilities
- `industrial-design-system`: Global minimalist industrial look (`data-style="industrial"`) with OLED black foundations, 1px crisp borders, Nothing Signal Red `#D71921` accent, and 16px dot-grid canvas.
- `ndot-typography-engine`: Embedded circular dot-matrix display font (`ndot57.woff2`) and high-tracking monospace stack for technical telemetry and headers.

### Modified Capabilities
- `bench-theme-engine`: Update System Settings modal to include the Interface Persona switcher and persist global style states.
- `unified-os-app-launcher`: Adapt launcher squircle grid to support industrial tile presentation with dot-matrix labels and status indicators.

## Impact

- **Frontend Assets**: New theme file `themes/nothing.css`, new font `webui/fonts/ndot57.woff2`, modifier rules in `webui/style.css`, and settings switcher in `webui/index.html` and `webui/lab.html`.
- **Packaging**: `tools/compile_themes.py` compiles `themes/nothing.css`; `tools/pack_webui.py` includes `fonts/ndot57.woff2` in `SYSTEM_FS`.
- **Zero Microcontroller Overhead**: 0 KB RAM on RP2350, < 6 KB gzipped flash in `SYSTEM_FS`.
