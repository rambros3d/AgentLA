## 1. Theme & Font Assets

- [x] 1.1 Create `themes/nothing.css` defining the Nothing OS palette (OLED black `#000000`, titanium `#0a0a0a`, `#1a1a1a` borders, Nothing Signal Red `#D71921`)
- [x] 1.2 Run `tools/compile_themes.py` to compile `webui/themes.css` and `webui/themes.json`
- [x] 1.3 Add subsetted `ndot57.woff2` circular dot-matrix font to `webui/fonts/ndot57.woff2`
- [x] 1.4 Add `@font-face` definition and `--font-display` custom properties in `webui/style.css`

## 2. Industrial Styling & Persona Modifiers

- [x] 2.1 Add `[data-style="industrial"]` rules to `webui/style.css` for 16px dot-grid canvas, 0.5rem box radiuses, and 1px crisp borders
- [x] 2.2 Style App Launcher squircle tiles for industrial presentation with dot-matrix titles and red status dots
- [x] 2.3 Implement high-tracking uppercase monospaced utility classes for hardware pin tags, memory offsets, and telemetry badges
- [x] 2.4 Style discrete/blocky progress indicators for storage and voltage meters

## 3. System Settings Switcher & FOUC Prevention

- [x] 3.1 Add Interface Persona toggle (`[ Industrial (Default) ]` vs `[ Bento ]`) to `#modal-settings` in `webui/index.html` and `webui/lab.html`
- [x] 3.2 Implement event listener in `webui/app.js` to toggle `data-style`, update DOM classes, and persist to `localStorage`
- [x] 3.3 Update instant pre-render script in `<head>` across all HTML files to apply `data-style` before layout paint

## 4. Packaging & Validation

- [x] 4.1 Update `tools/pack_webui.py` to pack `fonts/ndot57.woff2` into the 512 KB `SYSTEM_FS` image
- [x] 4.2 Verify persona switching and typography rendering in static demo mode
- [x] 4.3 Validate compilation and verify that flash footprint remains comfortably within `SYSTEM_FS`
