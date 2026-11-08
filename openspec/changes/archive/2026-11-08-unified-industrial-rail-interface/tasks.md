## 1. Typography & Font Engine

- [x] 1.1 Add 5x7 circular dot-matrix bitmaps for lowercase characters `a` through `z` to `tools/generate_ndot_font.py`
- [x] 1.2 Regenerate `webui/fonts/ndot57.woff2` and verify font file size and ASCII glyph coverage
- [x] 1.3 Update typography rules in `webui/style.css` to implement the 3-tier hybrid typography system (NDot headings, technical Monospace for metadata/tables/badges/buttons, clean Sans-Serif for body paragraphs)

## 2. Industrial Theme System & Light Mode

- [x] 2.1 Define the `industrial-light` DaisyUI theme tokens in `webui/themes.css` with white canvas (`#ffffff`), light surfaces (`#f5f5f5`), structural borders (`#e5e5e5`), black typography (`#0f0f0f`), and Signal Red accent (`#D71921`)
- [x] 2.2 Configure adaptive 16px dot-matrix background styling for both dark (`rgba(255, 255, 255, 0.12)`) and light (`rgba(0, 0, 0, 0.12)`) modes
- [x] 2.3 Permanently retire Bento styling: purge dead `.bento-card`, `.bento-*`, and `[data-style="bento"]` rules from `webui/style.css`
- [x] 2.4 Remove the Interface Persona select control from `#modal-settings` in `webui/index.html` and simplify theme hydration logic

## 3. Industrial Chamfered Rail Dock & Dockable Canvas Widgets

- [x] 3.1 Restyle `.floating-glass-dock` into the Industrial Chamfered Rail silhouette with 0.5rem corner radius, 1px structural border, hairline vertical dividers, and Signal Red active indicator pip
- [x] 3.2 Add "Pin to Workspace" action controls to both the Telemetry HUD and Flash Storage popovers
- [x] 3.3 Implement `#pinned-widgets-area` in `webui/index.html` workspace hero area with `localStorage` persistence under `agentla_pinned_widgets`
- [x] 3.4 Wire live telemetry and storage streams to update both dock popovers and pinned workspace widgets simultaneously

## 4. Industrial Storage Console & Packaging Verification

- [x] 4.1 Update `webui/files.html` headers, breadcrumbs, partition gauges, and inspector modals to the Industrial Storage Console format
- [x] 4.2 Verify WebUI bundling and partition headroom via `python3 tools/pack_webui.py`
- [x] 4.3 Verify `#![no_std]` firmware compilation via `cargo check --target thumbv8m.main-none-eabihf`
