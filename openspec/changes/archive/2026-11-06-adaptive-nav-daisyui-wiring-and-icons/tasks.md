## 1. WebUI Initialization & Utility Fixes

- [x] 1.1 Add `escapeHtml` utility at the top of `firmware/html/app.js` and wrap module initializers with error boundary logging so startup never halts.
- [x] 1.2 Fix MCP profile calls in `firmware/html/app.js` to adhere to JSON-RPC contract (`callMcp('cmd', 'profile_list')`, `profile_safe`, `profile_apply`, `profile_save`).
- [x] 1.3 Implement simulated hardware profile state and MCP mock handlers in `web/demo/mock-bridge.js` for demo mode.
- [x] 1.4 Verify Scope probe pin buttons (GP10–GP29) click-to-configure modal interaction, label sanitization, and real-time badge updates.

## 2. DaisyUI Emerald Theme, Depth, Noise & Token Engine

- [x] 2.1 Update `firmware/html/style.css` `[data-theme="light"]` with the exact DaisyUI Emerald v5 design tokens (base 100/200/300, primary content, borders, radiuses, depth, noise).
- [x] 2.2 Wire `--depth` token into dynamic neumorphic drop shadows, specular rim highlights, and button press active translations.
- [x] 2.3 Wire `--noise` token into a procedural SVG micro-texture matte finish overlay across instrument surfaces.
- [x] 2.4 Bind `--radius-box`, `--radius-field`, `--radius-selector`, and `var(--border)` across cards, inputs, buttons, and modals.
- [x] 2.5 Eliminate all hardcoded Slate palette hex codes in `.profile-dropdown-menu`, `.profile-pill`, and modal dialogs, binding 100% to DaisyUI tokens with Emerald Green (`--color-primary`) active accents.

## 3. Solar Duotone Navigation Icons

- [x] 3.1 Update desktop sidebar `<nav class="desktop-vertical-nav">` in `firmware/html/index.html` with Solar Duotone SVGs for Scope (`graph`), Adapters (`link-circle`), Debugger (`magnifier-bug`), and Console (`programming`).
- [x] 3.2 Update mobile bottom navigation `<nav class="primary-mode-nav">` in `firmware/html/index.html` with matching Solar Duotone SVGs.

## 4. Dynamic Content-Aware Bottom Navigation Bar

- [x] 4.1 Update `.primary-mode-nav` and `.sub-btn` in `firmware/html/style.css` to use content-aware flex scaling (`flex: 0 0 auto`).
- [x] 4.2 Implement compact mode (`< 580px`) accordion pill expansion: inactive tabs collapse to snug icon-only touch targets; active tab expands into an illuminated pill with icon + label.
- [x] 4.3 Implement roomy mode (`580px` to `899px`): display both icon and text label simultaneously for all navigation tabs with centered dock alignment.

## 5. Verification & Testing

- [x] 5.1 Run test suite (`pytest test/test_board_manifests.py test/test_protocol_decoders.py test/test_sigrok_export.py -v`).
- [x] 5.2 Validate JavaScript syntax for `app.js` and `mock-bridge.js` (`node -c`).
- [x] 5.3 Compile firmware binary and stage web assets into `pico_fsdata.inc` (`make -C build-work -j4 AgentLA`).
