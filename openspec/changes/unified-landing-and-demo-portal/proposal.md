## Why

The current demo setup under `website/demo/` relies on a fragile spiderweb of 13 individual file symlinks pointing back to `webui/`, `boards/demo/`, and `apps/`, plus accidental duplicate symlinks at the root of `website/`. Whenever a developer adds, renames, or restructures files in `webui/` (such as `terminal.html` or new app packages), the demo breaks unless symlinks are manually recreated. Furthermore, `mock-bridge.js` lives isolated inside `website/demo/` instead of `webui/`, even though `webui/index.html` expects it when running in demo mode. Finally, GitHub Pages deployment does not trigger on changes to `webui/`, leading to stale public demos.

Now that AgentLA has expanded to a multi-app platform with an integrated serial terminal, app store, and MCP integration, the public portal must maintain an up-to-date, interactive demo directly reusing `webui/` without manual symlink maintenance.

## What Changes

- **Move In-Browser Simulator to WebUI**: Relocate `mock-bridge.js` into `webui/mock-bridge.js` so `webui` can be tested in demo/simulation mode standalone in any browser.
- **Enforce Zero-Overhead Firmware LittleFS Packaging**: Update `tools/pack_webui.py` to explicitly exclude `mock-bridge.js` and test/demo-only files from the RP2350 LittleFS image, preserving all 512 KB flash headroom for firmware and apps.
- **Eliminate File-Level Symlink Clutter**: Remove the 13 individual symlinks inside `website/demo/` and all stray root symlinks in `website/`.
- **Implement Automated Portal Staging Pipeline (`tools/build_portal.py`)**: Create a cross-platform Python staging script that compiles `website/` (marketing landing page) and stages `webui/` at `/demo/` with demo board assets into an assembled output directory (`_site/`), complete with a local `--serve` preview server.
- **Interactive Inline Bezel on Landing Page**: Update `website/index.html` hero section to embed an interactive simulated workbench frame (`<iframe src="demo/?embed=1">`), accompanied by an "Expand to Fullscreen Workbench" button.
- **Embed Mode Optimization**: Support `?embed=1` in `webui/index.html` to hide redundant top brand bars when running embedded inside the landing page bezel.
- **Synchronize CI/CD Deployment**: Update `.github/workflows/deploy-pages.yml` to trigger on `website/**`, `webui/**`, `boards/demo/**`, and `apps/**`, building via `tools/build_portal.py`.

## Capabilities

### New Capabilities
- `demo-simulator-decoupling`: Governs the co-location of the in-browser simulator (`mock-bridge.js`) inside `webui/` for standalone browser testing while enforcing zero flash overhead in the embedded LittleFS firmware image.

### Modified Capabilities
- `static-portal-deployment`: Replaces manual file symlink structure with automated assembly pipeline (`tools/build_portal.py`), deploys `webui` cleanly under `/demo/`, and triggers CI builds on both marketing and webui changes.
- `hosted-marketing-splash`: Adds interactive hero workbench preview iframe (`?embed=1`) alongside product marketing features, 30-pin dynamic matrix, and local subnet device discovery.

## Impact

- **WebUI & Website Codebase**: Eliminates brittle symlinks in `website/` and `website/demo/`; adds `webui/mock-bridge.js`.
- **Tooling**: Introduces `tools/build_portal.py`; modifies `tools/pack_webui.py` to filter out demo/simulation files.
- **CI/CD**: Updates `.github/workflows/deploy-pages.yml` to trigger on `webui` and build using the new staging script.
- **Firmware Flash Impact**: No impact on RP2350 binary or LittleFS size; simulation scripts are strictly excluded from LittleFS packaging.
