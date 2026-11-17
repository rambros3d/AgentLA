## 1. Dashboard Layout Refactoring

- [x] 1.1 Restructure `#view-dashboard` in `webui/barefoc.html` into a responsive 12-column grid (`grid-cols-1 lg:grid-cols-12 gap-5`).
- [x] 1.2 Assemble the Left Column (`lg:col-span-5`) with primary telemetry HUD cards (compact 2-column / 3-column grid) and stacked Phase Voltages, Electrical Angles, and Modulation Mode diagnostic cards.
- [x] 1.3 Assemble the Right Column (`lg:col-span-7`) with the Interactive Radial Setpoint Dial card and Motion Control Modes card.
- [x] 1.4 Verify all element IDs remain intact for 100% JavaScript telemetry update and control compatibility.

## 2. Packaging, Live Deployment & Verification

- [x] 2.1 Package updated WebUI into system image and tarball using `tools/pack_webui.py`.
- [x] 2.2 Deploy `build/webui.tar` to the live AgentLA hardware probe at `10.10.10.10` via `POST /api/v1/system/webui/update`.
- [x] 2.3 Verify `http://10.10.10.10/motor` renders the two-column dashboard cleanly in landscape without vertical scrolling, with zero console errors.
- [x] 2.4 Run `pytest test/test_webui_assets.py` to confirm asset integrity and test suite pass.
