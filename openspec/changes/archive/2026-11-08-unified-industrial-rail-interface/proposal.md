## Why

AgentLA currently maintains a split aesthetic personality between the consumer-focused Bento style (rounded bubbles, pastel accents, drop shadows) and the industrial dot-matrix design language. Maintaining dual personas adds runtime stylesheet overhead, configuration complexity in settings, and visual inconsistency for a precision embedded hardware tool. By completely retiring the Bento persona and unifying on the Industrial design system, AgentLA achieves an uncompromising, hardware-authentic aesthetic with a precision chamfered rail dock, dockable canvas telemetry widgets, and full lowercase dot-matrix typography.

## What Changes

- **Retire Bento Persona**: Completely eliminate `data-style="bento"`, Bento-specific CSS classes, and the persona selector from the System Settings modal. Establish the Industrial aesthetic as the sole, permanent design language across all WebUI surfaces.
- **Industrial Light Mode (Technical Alabaster)**: Implement an authentic Industrial Light theme in DaisyUI (`#ffffff` canvas, `#f5f5f5` surfaces, `#e5e5e5` mechanical borders, `#0f0f0f` typography, inverted dark dot-matrix background, and Signal Red `#D71921` accent), seamlessly toggled with Dark mode via the top bar.
- **NDot Circular Dot-Matrix Lowercase Expansion**: Extend `tools/generate_ndot_font.py` with 5x7 circular dot bitmaps for lowercase characters `a` through `z`, generating a complete ASCII WOFF2 font (`~2.2 KB`) supporting mixed-case headings and badges.
- **Hybrid Monospace Typography System**: Establish strict typography layering: NDot circular matrix for headings/titles, technical uppercase Monospace with expanded tracking for parameters/badges/tables/buttons, and clean Sans-Serif for long explanatory text.
- **Industrial Chamfered Rail Dock**: Redesign the floating bottom dock from a 24px rounded pill into a sharp 0.5rem (`8px`) squircle box container with deep smoked black glass (`rgba(10, 10, 10, 0.90)`), 1px structural hairlines (`rgba(255, 255, 255, 0.12)`), vertical dividers, and Nothing Signal Red (`#D71921`) active indicator dots.
- **Dockable Canvas Telemetry & Storage Widgets**: Add a "Pin to Workspace" button to the Telemetry HUD and Device/Storage popovers, allowing live voltage/current/power/temperature/GPIO and Flash metrics to be mounted directly onto the home workbench canvas while preserving dock popover access.
- **Industrial Storage Console**: Align `webui/files.html` from the "Bento Storage Explorer" to the "Industrial Storage Console" with dot-matrix partition titles, monospaced breadcrumb headers (`root://user/apps/`), and segmented LED progress gauges.

## Capabilities

### New Capabilities
- `dockable-canvas-telemetry`: Live hardware telemetry and flash partition status widgets capable of being pinned directly onto the main workspace canvas or toggled via dock popovers, with state persisted in `localStorage`.

### Modified Capabilities
- `industrial-design-system`: Mandate the Industrial persona as the sole, non-optional design language across the entire WebUI, retiring Bento styling and removing dual-persona selector controls.
- `ndot-typography-engine`: Expand NDot57 dot-matrix font with lowercase glyphs `a-z` and define the hybrid monospace typography hierarchy.
- `unified-os-app-launcher`: Update floating dock specification to the Industrial Chamfered Rail silhouette with Signal Red active dots, hairline dividers, and workspace widget pinning integration.
- `bento-file-manager`: Update file manager specification to transition from Bento explorer layout to the Industrial Storage Console format.

## Impact

- `webui/style.css`: Remove dead Bento selectors (`.bento-card`, `.bento-*`, `[data-style="bento"]`); update floating dock to 0.5rem chamfered rail styling; apply hybrid monospace rules.
- `webui/index.html`: Remove style selector dropdown from `#modal-settings`; add pinned workspace telemetry/storage widget containers and pinning controls.
- `webui/files.html`: Update headers, breadcrumbs, and partition gauges to the Industrial Storage Console styling.
- `tools/generate_ndot_font.py`: Add 5x7 circular dot matrix bitmaps for lowercase `a-z`; regenerate `webui/fonts/ndot57.woff2`.
- `tools/pack_webui.py`: Ensure regenerated font and updated HTML/CSS assets pack cleanly into `SYSTEM_FS` LittleFS image.
