# Tasks: Unified Active Bench UX & Navigation

## 1. Structure and Responsive Layout (HTML & CSS)

- [x] 1.1 Refactor header navigation into two primary modes (`[ ⚡ Active Bench ]` and `[ 🎯 Target Station ]`) plus `[ 🔌 Pins ]` drawer trigger in `firmware/html/index.html` and `web/demo/index.html`.
- [x] 1.2 Implement `.active-bench-container` responsive layout in `firmware/html/style.css` and `web/demo/style.css`: desktop (≥1200px) side-by-side (canvas 65% / stimulus 35%), compact (<1200px) split-deck.
- [x] 1.3 Implement docked Stimulus Deck markup and tabs (Bus Masters, Stimulus REPL, Pattern Gen, Pin Toggler) in `firmware/html/index.html` and `web/demo/index.html`.
- [x] 1.4 Implement slide-out Pin Matrix Drawer markup and CSS with smooth translate transitions and pin conflict badges.

## 2. Dynamic Behavior and Synchronized Send & Sniff (JavaScript)

- [x] 2.1 Implement primary mode switching logic between Active Bench and Target Station in `firmware/html/app.js` and `web/demo/app.js`.
- [x] 2.2 Wire slide-out Pin Matrix Drawer open/close toggles (`[ 🔌 Pins ]` button, overlay backdrop, and `P` hotkey).
- [x] 2.3 Implement the `[x] Auto-Sniff on Transmit` toolbar toggle pill and integrate into bus master transmission handlers.
- [x] 2.4 Implement waveform canvas auto-zoom and centering when transmitted packets are captured.
- [x] 2.5 Wire `ResizeObserver` on the canvas container to adaptively redraw waveforms during responsive breakpoint changes without losing zoom/pan state.

## 3. Firmware Build & Verification

- [x] 3.1 Regenerate `generated/pico_fsdata.inc` using Pico HTTPD content generator.
- [x] 3.2 Build firmware cleanly (`make -C build-work -j4 AgentLA`) and verify zero linker overflow.
- [x] 3.3 Execute test suite (`pytest test/test_protocol_decoders.py test/test_sigrok_export.py`) to confirm protocol decoding integrity.
- [x] 3.4 Sync specs to `openspec/specs/` and archive the change.
