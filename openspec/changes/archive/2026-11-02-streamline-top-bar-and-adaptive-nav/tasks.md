## 1. Top Bar Markup & Asset Decluttering

- [x] 1.1 Remove `#btn-webserial-connect` and the `← Portal` anchor element from `firmware/html/index.html`.
- [x] 1.2 Add the Open Source Initiative (OSI) keyhole icon button link immediately before `#btn-theme-toggle` in `firmware/html/index.html`.
- [x] 1.3 Clean up the dynamic demo loader script in `firmware/html/index.html` and dead event listeners in `web/demo/mock-bridge.js`.

## 2. Desktop Left Vertical Sidebar & Responsive Shell

- [x] 2.1 In `firmware/html/index.html`, restructure the workspace layout to introduce the Desktop Left Vertical Sidebar (`.desktop-vertical-nav`) hosting the 5 primary mode buttons (`Active Bench`, `Adapters`, `Debugger`, `Pin Matrix`, `Console`).
- [x] 2.2 In `firmware/html/index.html`, wrap mode text labels with `.btn-label-vertical` inside the vertical navigation tabs.
- [x] 2.3 In `firmware/html/style.css`, style `.desktop-vertical-nav` for viewports $\ge 900\text{px}$, applying `transform: rotate(-90deg); display: inline-block; white-space: nowrap;` to rotate the word label itself counter-clockwise by 90 degrees.
- [x] 2.4 In `firmware/html/style.css`, configure responsive switching so `.desktop-vertical-nav` is active on desktop ($\ge 900\text{px}$) and hidden on mobile/portrait ($< 900\text{px}$), where the bottom navigation dock activates.
- [x] 2.5 Style the OSI project link button in `firmware/html/style.css` to match top-bar icon buttons (`32px` button, `18px` icon, border, hover effect).

## 3. Console Tab UI: Agent Guide, Theming & Floating Clear Button

- [x] 3.1 In `firmware/html/index.html`, replace `#btn-cmd-clear` in `.matrix-header-actions` with the `#btn-agent-guide` button.
- [x] 3.2 In `firmware/html/index.html`, add `#btn-cmd-clear` as a compact badge button with trash icon and "Clear" text in the bottom-right corner of `.panel.terminal-display`.
- [x] 3.3 In `firmware/html/index.html`, add the `#modal-agent-guide` markup with unified MCP instructions (JSON snippet, transport URLs, agent destination guide, key tools, and prompt examples).
- [x] 3.4 In `firmware/html/style.css`, style `.terminal-clear-badge-btn` with absolute positioning, subtle opacity, and hover feedback.
- [x] 3.5 In `firmware/html/style.css`, add styling for `#modal-agent-guide` and its code snippets.

## 4. Console Text Wrapping, Viewport Containment & Click-to-Copy

- [x] 4.1 In `firmware/html/style.css`, add `white-space: pre-wrap; overflow-wrap: anywhere; word-break: break-word; max-width: 100%;` to `.output-content` and `#cmd-output-content`.
- [x] 4.2 In `firmware/html/style.css`, set `overflow-x: hidden; min-width: 0; max-width: 100%; box-sizing: border-box;` on `.panel.terminal-display`.
- [x] 4.3 In `firmware/html/style.css`, update `.console-main-grid` to use `grid-template-columns: minmax(0, 2fr) minmax(0, 1fr); min-width: 0; max-width: 100%;` (and `minmax(0, 1fr)` at $< 900\text{px}$).
- [x] 4.4 In `firmware/html/style.css`, set `min-width: 0; max-width: 100%;` on `.console-terminal-card` and `.console-cheatsheet-card`.
- [x] 4.5 In `firmware/html/style.css`, define semantic console line classes using OKLCH tokens (`.console-line-input` using `var(--color-primary)`, `.console-line-output` using `var(--color-base-content)`, `.console-line-success` using `var(--color-success)`, `.console-line-error` using `var(--color-error)`).
- [x] 4.6 In `firmware/html/style.css`, define styles for `.console-line:hover` and the temporary floating inline `.line-copied-badge`.

## 5. JavaScript Controllers & Handlers

- [x] 5.1 In `firmware/html/app.js`, update `appendLog` to generate individual `.console-line` elements styled with semantic classes.
- [x] 5.2 In `firmware/html/app.js`, attach click-to-copy handler on each `.console-line` with clipboard copy and inline "Copied!" badge feedback.
- [x] 5.3 In `firmware/html/app.js`, wire up `#btn-agent-guide` to open `#modal-agent-guide`, and wire close handlers (✕ button, backdrop click, Escape key, "Got It" button).
- [x] 5.4 In `firmware/html/app.js`, wire up `#btn-copy-mcp-config` to copy the universal JSON configuration to the system clipboard with feedback.
- [x] 5.5 In `firmware/html/app.js`, wire `#btn-cmd-clear` in `.terminal-display` to clear output content.
- [x] 5.6 In `firmware/html/app.js`, ensure mode switching updates both desktop vertical sidebar tabs and mobile bottom nav tabs seamlessly.

## 6. Verification & Testing

- [x] 6.1 Build firmware and re-stage web assets into `pico_fsdata.inc` via `make -C build-work -j4 AgentLA`.
- [x] 6.2 Run automated pytest test suite (`pytest test/test_board_manifests.py test/test_protocol_decoders.py test/test_sigrok_export.py -v`).
- [x] 6.3 Validate responsive behavior across wide desktop ($\ge 900\text{px}$) with the vertical sidebar and -90° rotated word labels, and mobile portrait ($< 900\text{px}$) with the bottom navigation dock.
- [x] 6.4 Validate long unbroken hex and JSON strings in the console wrap cleanly without expanding the console card or creating viewport horizontal scrollbars.
