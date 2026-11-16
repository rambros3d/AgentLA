## 1. CSS Framework & Adaptive Rail Layout Engine

- [x] 1.1 Implement `.app-shell` flex container and `.app-nav-rail` rules in `webui/style.css` (48px vertical sidebar on >= 900px, 48px horizontal bottom bar on < 900px)
- [x] 1.2 Update `.nav-rail-btn` styles, surface active states, and right-flying desktop tooltips without dot indicator pips
- [x] 1.3 Prune legacy artificial bottom padding hacks (`padding-bottom: 72px/84px`) from `.launcher-workspace`, `.main-workspace`, `.adapters-container`, and `.scope-container`
- [x] 1.4 Update `#telemetry-hud` and `#storage-glass-popover` positioning for desktop sidebar (anchored to the right) and mobile bottom bar (anchored above)

## 2. HTML Markup Migration

- [x] 2.1 Refactor navigation markup in `webui/index.html` to position `<nav id="app-nav-rail">` inside `.app-shell` as an in-flow flex sibling to `#main-workspace`
- [x] 2.2 Refactor navigation markup in `webui/lab.html` to position `<nav id="app-nav-rail">` inside `.app-shell` as an in-flow flex sibling to `#main-workspace`
- [x] 2.3 Refactor navigation markup in `webui/files.html` to position `<nav id="app-nav-rail">` inside `.app-shell`

## 3. JavaScript Controllers & Bridge Alignment

- [x] 3.1 Update `webui/app.js` navigation and popover toggle logic to support adjacent rail popover docking
- [x] 3.2 Update `webui/index.html` navigation and popover toggle logic to support adjacent rail popover docking
- [x] 3.3 Update dynamic navigation mounting and popover positioning in `webui/agentla-bridge.js`

## 4. Verification & Packaging

- [x] 4.1 Run `tools/pack_webui.py` to confirm LittleFS `webui.img` and `webui.tar` build with >= 35% partition headroom
- [x] 4.2 Verify responsive layout across desktop (>= 900px) and mobile (< 900px) with strictly zero emojis, no CSS dot pips, and flush edge anchoring
