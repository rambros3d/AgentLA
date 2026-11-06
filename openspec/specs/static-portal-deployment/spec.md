# static-portal-deployment Specification

## Purpose
Governs the separation between hosted public marketing assets (`web/`) and embedded probe firmware, enforcing decoupled LittleFS flash storage and pre-compressed static asset delivery.

## Requirements

### Requirement: Firmware Flash Isolation & Pre-Compressed Static Streaming
All marketing assets, simulation data, and hosted scripts SHALL reside in the `web/` directory and MUST NOT be compiled into firmware ROM. WebUI assets (`web-ui/`) SHALL be packaged into a standalone 512 KB LittleFS binary filesystem image flashed to Partition 1 (`0x00190000`) rather than compiled as `include_str!` string literals into the firmware executable.

The System WebUI filesystem image SHALL store pre-compressed `.gz` assets (`index.html.gz`, `apps.html.gz`, `style.css.gz`, `themes.css.gz`, `app.js.gz`). The HTTP server SHALL serve these assets with `Content-Encoding: gzip` when requested with `Accept-Encoding: gzip`, reducing total LittleFS storage footprint to under 200 KB while accelerating USB transfer rates.

#### Scenario: Compiling RP2350 firmware binary
- **WHEN** building the embedded firmware with `cargo build --release`
- **THEN** `firmware-rs` contains only machine code and runtime logic (~350 KB) without bundled HTML/JS/CSS assets in its `.rodata` section

#### Scenario: Serving Gzip-Compressed System WebUI
- **WHEN** a client HTTP request requests `/style.css` with `Accept-Encoding: gzip`
- **THEN** the HTTP server streams `style.css.gz` directly from LittleFS `SYSTEM_FS` with response header `Content-Encoding: gzip` and `Content-Type: text/css`

#### Scenario: Triggering CI deployment
- **WHEN** changes to `web/` are pushed to the repository default branch
- **THEN** GitHub Pages action deploys the static files directly to the public web hosting URL with zero manual steps
