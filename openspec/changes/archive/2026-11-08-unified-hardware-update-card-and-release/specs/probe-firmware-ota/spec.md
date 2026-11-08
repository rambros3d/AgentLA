## ADDED Requirements

### Requirement: All-in-One Multi-Segment BOOTSEL UF2 Distribution
The build and release system SHALL package an All-in-One UF2 binary (`agentla-pico2.uf2`) containing three disjoint segments: Invariant Bootloader at `0x10000000` (8 KB), Firmware Slot A at `0x10004000` (up to 1,024 KB), and WebUI LittleFS filesystem at `0x10204000` (512 KB), allowing single-file complete offline flashing via RP2350 USB BOOTSEL mode.

#### Scenario: Flashing All-in-One UF2 via USB BOOTSEL
- **WHEN** user copies `agentla-pico2.uf2` onto the `RPI-RP2` USB drive
- **THEN** the RP2350 bootrom writes bootloader, firmware, and WebUI filesystem to their respective offsets, rebooting immediately into a fully functional system.

### Requirement: GitHub Actions Automated Tagged Release Workflow
The repository SHALL contain a GitHub Actions workflow (`.github/workflows/release.yml`) triggered on tag pushes (`v*`) and manual workflow dispatch that compiles `firmware-rs`, generates WebUI LittleFS assets, packages the multi-segment UF2 and OTA binaries, and creates a GitHub Release with `agentla-pico2.uf2`, `agentla-pico2-ota.bin`, and `webui.tar`.

#### Scenario: Pushing a release tag to GitHub
- **WHEN** tag `v0.1.1` is pushed to `rambros3d/AgentLA`
- **THEN** the GitHub Actions workflow builds all release artifacts and creates release `v0.1.1` with the required binaries attached.
