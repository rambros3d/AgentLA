## 1. Mode Renaming and Navigation Updates

- [ ] 1.1 Rename "Active Bench" to "Scope" across desktop left vertical navigation (-90° rotated word label `SCOPE`), mobile bottom dock, tab headers, and keyboard shortcut indicators
- [ ] 1.2 Update view routing and mode state logic in `firmware/html/app.js` to handle `data-mode="scope"`, URL hash/query parameters, and active navigation class toggling

## 2. Interactive Top Probe Pins Bar

- [ ] 2.1 Add top pins bar HTML container (`#scope-pins-bar`) above the Scope window with buttons for all capture pins `GP10` through `GP29`
- [ ] 2.2 Style top pins bar in `firmware/html/style.css` with pin buttons, channel status dots, ADC diamond badges `◈`, and hover/active states
- [ ] 2.3 Implement pin bar rendering and click handler in `firmware/html/app.js` to open the pin configuration modal with the clicked pin pre-selected

## 3. Interactive Pin Configuration Modal

- [ ] 3.1 Construct `#modal-pin-config` dialog markup in `firmware/html/index.html` with pin mode selector (`Digital Input`, `Analog Input` for GP26–GP29 only, `Released / High-Z`), channel label input, 18-color swatch grid, and trigger rule controls
- [ ] 3.2 Add CSS styling for `#modal-pin-config` in `firmware/html/style.css` including neumorphic modal framing and the 6 × 3 color swatch grid with selection rings
- [ ] 3.3 Implement modal state and controller in `firmware/html/app.js`: populate pin data, enforce analog eligibility for GP26–GP29, handle 18-color swatch selection, configure logical digital triggers (without voltage thresholds) vs analog voltage threshold comparators, and save back to instrument configuration

## 4. Unified MSO Window & Draggable Splitter

- [ ] 4.1 Restructure Scope observation DOM in `firmware/html/index.html` to create `#scope-mso-container` housing `#scope-digital-subwindow`, `#scope-mso-splitter`, and `#scope-analog-subwindow`
- [ ] 4.2 Style `#scope-mso-container` and `#scope-mso-splitter` in `firmware/html/style.css` with tactile draggable grip handle `⋯`, cursor styling, and dynamic layout supporting 100% digital height when analog channels are empty
- [ ] 4.3 Implement draggable splitter mouse/touch handlers in `firmware/html/app.js` with minimum sizing constraints and `localStorage` height persistence
- [ ] 4.4 Update canvas rendering loops: digital scope renders multi-lane stacked traces with inline frequency/duty measurements; analog scope renders superimposed channels on shared 8 × 10 division graticule with calibrated Volts/div and Time/div

## 5. Integrated Protocol Decoder Bar & Packet Drawer

- [ ] 5.1 Place integrated decoder bar directly above digital scope lanes in `firmware/html/index.html` (`#scope-decoder-bar`) with protocol selector dropdown, channel mapping pills, and `[ 📋 Packet Table ]` button
- [ ] 5.2 Add styles for decoder bar, in-canvas packet bubbles, hover tooltips, and slide-out packet table drawer in `firmware/html/style.css`
- [ ] 5.3 Implement decoder UI controller in `firmware/html/app.js`: protocol selection, channel mapping update, packet tooltip rendering on canvas hover, and table drawer toggle with search filter

## 6. Clustered Scope Toolbar & Collapsible Stimulus Drawer

- [ ] 6.1 Reorganize Scope top toolbar in `firmware/html/index.html` into 3 semantic clusters (Capture, Analysis, Bank Claim) and integrate floating zoom controls (`+`, `−`, `Fit`) on the canvas overlay
- [ ] 6.2 Convert bottom Stimulus Deck (Signal Generator) into a collapsible footer drawer with minimized single-line status bar (`[ ⚡ Signal Generator: GP12 @ 12.0 kHz (50% PWM) ▴ ]`) and expanding panel
- [ ] 6.3 Add CSS transitions and styling for clustered toolbar and collapsible stimulus drawer in `firmware/html/style.css`
- [ ] 6.4 Implement drawer toggle and status bar update logic in `firmware/html/app.js`

## 7. Verification and Testing

- [ ] 7.1 Verify WebUI rendering, responsive resizing, pin configuration modal, color selection, and splitter behavior in browser test environment
- [ ] 7.2 Run regression test suite (`pytest test/test_board_manifests.py test/test_protocol_decoders.py test/test_sigrok_export.py`) and verify firmware build compilation (`make -C build-work -j4 AgentLA`)
