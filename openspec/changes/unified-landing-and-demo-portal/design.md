## Context

AgentLA delivers an embedded web user interface (`webui/`) running inside RP2350 flash (LittleFS partition) for hardware logic analysis, flasher/debugger controls, and serial communications. Simultaneously, a public marketing and documentation landing site (`website/`) is hosted via GitHub Pages.

Previously, `website/demo/` mirrored `webui/` through 13 separate file-level symlinks and loose root symlinks (`website/apps.html`, `website/terminal.html`). This caused persistent breakage whenever new files were added to `webui/`. Additionally, the in-browser simulator (`mock-bridge.js`) was located solely in `website/demo/`, leaving `webui/` unable to run standalone in simulated environments.

## Goals / Non-Goals

**Goals:**
- Eliminate individual file symlinks between `website/` and `webui/`.
- Co-locate `mock-bridge.js` within `webui/` so the WebUI can run in demo/simulation mode standalone without external dependencies.
- Update `tools/pack_webui.py` to strictly ignore `mock-bridge.js` and simulation assets, guaranteeing 0 bytes of flash bloat in the RP2350 LittleFS filesystem.
- Create an automated cross-platform Python staging script `tools/build_portal.py` to assemble the production static site (`_site/`) with `--output` and `--serve` flags for local preview.
- Enhance `website/index.html` with an interactive simulated hardware bezel iframe (`<iframe src="demo/?embed=1">`) directly inside the marketing hero, plus an "Expand to Fullscreen Workbench" button.
- Optimize `webui/index.html` to support `?embed=1` mode by hiding redundant headers and maximizing waveform canvas area.
- Update GitHub Actions workflow (`deploy-pages.yml`) to trigger on both `website/` and `webui/` changes.

**Non-Goals:**
- Replacing DaisyUI / Tailwind design language with a third-party framework.
- Modifying the embedded Rust HTTP server in `firmware-rs`.
- Changing how LittleFS partition offsets or UF2 binaries are constructed for hardware flashing.

## Decisions

### Decision 1: Single Source of Truth in `webui/` with LittleFS Packaging Exclusion
- **Choice**: Place `mock-bridge.js` directly in `webui/` and explicitly exclude `mock-bridge.js` and demo assets in `tools/pack_webui.py`.
- **Alternatives considered**:
  - *Keep mock-bridge in website*: Rejected because `webui/index.html` itself references `mock-bridge.js` when running locally or on static hosts. Keeping it external caused 404 errors.
  - *Compile mock-bridge into firmware LittleFS*: Rejected because 43 KB of uncompressed mock logic wastes ~8.4% of the 512 KB LittleFS partition that is needed for user applications and real drivers.

### Decision 2: Automated Python Staging (`tools/build_portal.py`) over OS Symlinks
- **Choice**: Implement a standalone Python script `tools/build_portal.py` that copies `website/` to the root of `_site/` and `webui/` into `_site/demo/`, alongside `boards/demo/` assets.
- **Alternatives considered**:
  - *Directory symlink `website/demo -> ../webui`*: Directory symlinks break on default Windows checkouts (requiring Developer Mode / SeCreateSymbolicLinkPrivilege) and can fail on certain static hosting platforms if not dereferenced.
  - *Vite / Webpack bundler*: Rejected because AgentLA uses pure vanilla ES modules and static HTML/CSS/JS with zero Node.js build dependencies.

### Decision 3: Hero Interactive Bezel with Compact Embed Support (`?embed=1`)
- **Choice**: Embed an interactive device preview window in `website/index.html` using `<iframe src="demo/?embed=1">`, while providing a dedicated fullscreen link to `/demo/`.
- **Alternatives considered**:
  - *Static screenshot or GIF in hero*: Misses the chance to immediately prove to visiting embedded engineers that the 100 MS/s logic analyzer and protocol decoders run live in the browser.
  - *Full WebUI directly on root `/`*: Loses the marketing narrative, feature cards, 30-pin domain matrix, and subnet probe connectivity tools.

## Risks / Trade-offs

- **[Risk]** `mock-bridge.js` accidentally packaged into firmware LittleFS partition.  
  → **Mitigation**: Add unit test in `test/test_webui_assets.py` verifying that `tools/pack_webui.py` skips `mock-bridge.js` and LittleFS image contains 0 bytes of mock simulator code.

- **[Risk]** Iframe in hero slows down initial landing page rendering on low-power devices.  
  → **Mitigation**: Add `loading="lazy"` attribute to iframe, and restrict automatic waveform rendering until visible.

- **[Risk]** Theme mismatch between landing page and embedded iframe.  
  → **Mitigation**: Both pages share the exact DaisyUI `agentla-theme` LocalStorage key, ensuring seamless dark/light synchronization.
