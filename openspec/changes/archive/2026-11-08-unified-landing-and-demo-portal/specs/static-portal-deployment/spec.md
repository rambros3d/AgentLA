## MODIFIED Requirements

### Requirement: Firmware Flash Isolation & Pre-Compressed Static Streaming
All marketing assets, simulation data, and hosted scripts SHALL reside in the `website/` directory and MUST NOT be compiled into firmware ROM. WebUI assets (`webui/`) SHALL be packaged into a standalone 512 KB LittleFS binary filesystem image flashed to Partition 1 (`0x00190000`) rather than compiled as `include_str!` string literals into the firmware executable.

The System WebUI filesystem image SHALL store pre-compressed `.gz` assets (`index.html.gz`, `apps.html.gz`, `style.css.gz`, `themes.css.gz`, `app.js.gz`). The HTTP server SHALL serve these assets with `Content-Encoding: gzip` when requested with `Accept-Encoding: gzip`, reducing total LittleFS storage footprint to under 200 KB while accelerating USB transfer rates.

#### Scenario: Compiling RP2350 firmware binary
- **WHEN** building the embedded firmware with `cargo build --release`
- **THEN** `firmware-rs` contains only machine code and runtime logic (~350 KB) without bundled HTML/JS/CSS assets in its `.rodata` section

#### Scenario: Serving Gzip-Compressed System WebUI
- **WHEN** a client HTTP request requests `/style.css` with `Accept-Encoding: gzip`
- **THEN** the HTTP server streams `style.css.gz` directly from LittleFS `SYSTEM_FS` with response header `Content-Encoding: gzip` and `Content-Type: text/css`

#### Scenario: Triggering CI deployment
- **WHEN** changes to `website/`, `webui/`, `boards/demo/`, or `apps/` are pushed to the repository default branch
- **THEN** GitHub Pages action executes `tools/build_portal.py` and deploys the static staging artifact to GitHub Pages with zero broken symlinks

## ADDED Requirements

### Requirement: Cross-Platform Portal Staging Pipeline
The repository SHALL provide a Python utility script `tools/build_portal.py` that stages the static public portal by copying `website/` into the output root and assembling `webui/`, `boards/demo/`, and catalog assets under `demo/`, supporting an optional `--serve [PORT]` flag for local preview without symlink dependencies.

#### Scenario: Assembling static portal distribution
- **WHEN** running `python3 tools/build_portal.py --output _site`
- **THEN** the script creates `_site/` with `index.html` at the root and a fully functional standalone demo under `_site/demo/`, including `.nojekyll`
