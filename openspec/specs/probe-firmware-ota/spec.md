# probe-firmware-ota Specification

## Purpose
Provides an in-service True A/B dual-slot firmware OTA update subsystem for the AgentLA probe, enabling live background streaming of candidate images over CDC-NCM Ethernet, pre-boot cryptographic integrity checks, atomic candidate reboot, and automatic 15-second hardware watchdog rollback without touching LittleFS filesystems or requiring physical BOOTSEL intervention.

## Requirements

### Requirement: True A/B dual-slot firmware partitioning
The system SHALL divide executable flash space into two symmetric 1.2 MB execution slots: Slot A (`0x00004000` to `0x00130000`) and Slot B (`0x00130000` to `0x00260000`), mediated by an initial 16 KB Boot Table (`0x00000000` to `0x00004000`).

#### Scenario: Active slot execution
- **WHEN** the probe boots normally
- **THEN** it executes from the currently confirmed active slot without modifying or erasing the alternate slot.

### Requirement: Streaming OTA REST API
The system SHALL expose an HTTP endpoint `POST /api/v1/system/firmware/ota` that accepts chunked binary uploads and streams the payload directly into the inactive slot's flash sectors.

#### Scenario: Streaming update to inactive slot
- **WHEN** an HTTP client streams a valid firmware image to `POST /api/v1/system/firmware/ota`
- **THEN** the server writes incoming blocks directly into the inactive slot while the active slot continues processing network traffic, returning HTTP 200 upon complete verification.

#### Scenario: Reject oversized image
- **WHEN** an incoming image exceeds 1,228,800 bytes (1.2 MB)
- **THEN** the server aborts the stream, erases partial sectors, and returns `HTTP 413 Payload Too Large`.

### Requirement: Pre-boot image integrity and compatibility
The system SHALL verify image magic header (`AGLA_FW`), target board identity, and SHA-256 / CRC-32 checksums before marking an inactive slot as candidate boot.

#### Scenario: Invalid magic or corrupted checksum
- **WHEN** an uploaded binary has a corrupted header or mismatched checksum
- **THEN** the system rejects the candidate switch, leaves the active slot unchanged, and returns `HTTP 400 Bad Request`.

### Requirement: Atomic candidate switch and automated watchdog rollback
The system SHALL boot a newly programmed slot as a candidate with a 15-second hardware watchdog timeout, automatically rolling back to the previous known-good slot if self-test confirmation is not completed.

#### Scenario: Successful self-test confirmation
- **WHEN** the candidate slot boots, initializes network, and calls `ota_confirm()` within 15 seconds
- **THEN** the boot table marks the candidate slot permanently active.

#### Scenario: Watchdog rollback on boot failure
- **WHEN** the candidate slot hangs, crashes, or panics before calling `ota_confirm()`
- **THEN** the hardware watchdog resets the core, and the boot manager reverts boot execution to the previously confirmed slot.

### Requirement: Decommission runtime BOOTSEL command
The system SHALL remove the runtime BOOTSEL reboot endpoint (`GET /api/v1/system/bootsel`) and MCP tool (`system_reboot_bootsel`), preventing accidental entry into unmanaged ROM mass storage mode.

#### Scenario: Attempting to call legacy bootsel endpoint
- **WHEN** a client sends an HTTP request to `GET /api/v1/system/bootsel`
- **THEN** the server returns `HTTP 404 Not Found` and does not reboot into ROM bootloader mode.

### Requirement: Developer Soft BOOTSEL Reboot
The system SHALL expose an HTTP endpoint `POST /api/v1/system/bootsel` that invokes `embassy_rp::rom_data::reset_to_usb_boot(0, 0)` on the RP2350 target, placing the board into USB mass storage bootloader mode without requiring physical BOOTSEL button presses or power cycling.

#### Scenario: Executing soft bootsel reboot
- **WHEN** an authorized client sends `POST /api/v1/system/bootsel`
- **THEN** the firmware flushes pending logs and reboots the RP2350 core into the USB ROM bootloader mode.

### Requirement: Developer LittleFS Partition Formatting
The system SHALL expose an HTTP endpoint `POST /api/v1/system/fs/format` accepting a JSON payload specifying `"partition": "system"` or `"partition": "user"` to reformat the respective LittleFS flash area in place.

#### Scenario: Formatting LittleFS system partition
- **WHEN** a client sends `POST /api/v1/system/fs/format` with `{"partition": "system"}`
- **THEN** the firmware unmounts `SYSTEM_FS`, erases the flash sectors from `0x10270000` to `0x102F0000`, initializes a blank LittleFS structure, and returns HTTP 200 OK.

### Requirement: Debugprobe Hardware Verification
The system SHALL be verified on live hardware using an attached Raspberry Pi Debugprobe (`2e8a:000c`) communicating via SWD with the AgentLA RP2350 target.

#### Scenario: SWD verification of bootloader and slot vectoring
- **WHEN** the 1st-stage bootloader executes on the RP2350 target
- **THEN** the Debugprobe inspects hardware registers confirming that Cortex-M33 VTOR is positioned at the selected slot (`0x10004000` for Slot A or `0x10130000` for Slot B) and execution begins at the target slot's `Reset_Handler`.

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
