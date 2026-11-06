## MODIFIED Requirements

### Requirement: Firmware Flash Isolation
All marketing assets, simulation data, and hosted scripts SHALL reside in the `web/` directory and MUST NOT be compiled into firmware ROM. WebUI assets (`web-ui/`) SHALL be packaged into a standalone 512 KB LittleFS binary filesystem image flashed to Partition 1 (`0x00190000`) rather than compiled as `include_str!` string literals into the firmware executable.

#### Scenario: Compiling RP2350 firmware binary
- **WHEN** building the embedded firmware with `cargo build --release`
- **THEN** `firmware-rs` contains only machine code and runtime logic (~350 KB) without bundled HTML/JS/CSS assets in its `.rodata` section

#### Scenario: Triggering CI deployment
- **WHEN** changes to `web/` are pushed to the repository default branch
- **THEN** GitHub Pages action deploys the static files directly to the public web hosting URL with zero manual steps
