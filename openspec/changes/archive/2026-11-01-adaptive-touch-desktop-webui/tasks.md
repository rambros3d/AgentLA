## 1. Asset & Build Cleanup

- [x] 1.1 Remove WinBox dependencies (`winbox.js`, `winbox.min.css`) from `firmware/html/`
- [x] 1.2 Update `CMakeLists.txt` `pico_set_lwip_httpd_content` asset file list to remove WinBox references

## 2. Adaptive Workspace Shell & Navigation

- [x] 2.1 Refactor `firmware/html/index.html` structure into semantic header, main workspace container, drawer/sheet containers, and navigation bars
- [x] 2.2 Implement responsive navigation in `firmware/html/style.css` (Bottom Navigation Bar with $\ge 48\text{px}$ tap targets on mobile, Left Rail on desktop $\ge 900\text{px}$) using dynamic viewport units (`100dvh`, `100dvw`)
- [x] 2.3 Implement navigation controller in `firmware/html/app.js` routing views and updating active tab states

## 3. Unified Dashboard Overview

- [x] 3.1 Build Dashboard view in `firmware/html/index.html` with probe health metrics and 1-tap instrument launch tiles
- [x] 3.2 Implement interactive 30-pin domain allocation widget (`GPIO 0–29`) showing real-time pin claims and function roles
- [x] 3.3 Wire live telemetry polling and SSE event updates into the Dashboard in `firmware/html/app.js`

## 4. Dual-Slot Split View & Presets

- [x] 4.1 Implement 2-slot CSS grid container in `firmware/html/style.css` supporting 1-pane focus mode and 2-pane split mode (side-by-side on wide screens, stacked on portrait)
- [x] 4.2 Implement slot manager in `firmware/html/app.js` with independent tool mounting, header controls, and slot swap
- [x] 4.3 Add 1-tap quick split presets: "Bus Debug" (Logic + Adapter) and "Target Bring-Up" (Flasher + Terminal)

## 5. Touch-Friendly Inspectors & Waveform Gestures

- [x] 5.1 Implement slide-up Bottom Sheet (touch/mobile) and slide-out Right Drawer (desktop) for tool settings and pin configuration
- [x] 5.2 Implement native multi-touch pointer gesture handler (`touch-action: none`) for the Logic Analyzer canvas supporting pinch-zoom, two-finger pan, and double-tap zoom reset
- [x] 5.3 Add `ResizeObserver` on instrument slots to dynamically adapt canvas resolution and prevent waveform stretching

## 6. Theme Engine & Validation

- [x] 6.1 Implement CSS custom property theming in `firmware/html/style.css` for Deep Dark (`#08111d`) and Light Theme
- [x] 6.2 Add theme switcher toggle in the top bar with `localStorage` persistence in `firmware/html/app.js`
- [x] 6.3 Rebuild firmware, compile HTTPD file system data (`pico_fsdata.inc`), and verify the complete application builds with zero errors
