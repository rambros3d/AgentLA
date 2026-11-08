## 1. WebUI Simulator Co-location & Firmware Flash Guard

- [x] 1.1 Move `mock-bridge.js` from `website/demo/` to `webui/mock-bridge.js`.
- [x] 1.2 Update `tools/pack_webui.py` to explicitly exclude `mock-bridge.js` and simulation assets from LittleFS image generation.
- [x] 1.3 Add test in `test/test_webui_assets.py` verifying that `mock-bridge.js` exists in `webui/` and is excluded from LittleFS packaging.

## 2. Automated Staging Pipeline (`tools/build_portal.py`)

- [x] 2.1 Implement `tools/build_portal.py` to stage `website/` as portal root, `webui/` as `demo/`, `boards/demo/*` as board profile, and `.nojekyll`.
- [x] 2.2 Add `--serve` argument to `tools/build_portal.py` for local development preview with proper MIME-types.
- [x] 2.3 Remove the 13 individual symlinks from `website/demo/` and loose symlinks from `website/` root.

## 3. Marketing Landing Page & Embed Mode Integration

- [x] 3.1 Update `webui/index.html` to recognize `?embed=1` and hide top bar branding for compact frame rendering.
- [x] 3.2 Add interactive device preview bezel with `<iframe src="demo/?embed=1">` into the hero section of `website/index.html`.
- [x] 3.3 Style the device frame and "Launch Fullscreen Workbench →" CTA buttons in `website/landing.css`.
- [x] 3.4 Verify cross-page theme synchronization between landing page and demo workbench.

## 4. CI/CD & Verification

- [x] 4.1 Update `.github/workflows/deploy-pages.yml` trigger paths (`webui/**`, `website/**`, `apps/**`) and assemble with `tools/build_portal.py`.
- [x] 4.2 Execute `python3 tools/build_portal.py --output _site` and verify all assets and links resolve cleanly without 404 errors.
