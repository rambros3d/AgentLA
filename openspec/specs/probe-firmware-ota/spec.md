# probe-firmware-ota Specification

## Purpose
Provides firmware OTA updates for the AgentLA probe in a fixed three-image flash layout. The preferred flow stages an image into USER_FS as a LittleFS file and reboots so the recovery app verifies and installs it into the 1 MB main application slot. A legacy in-place HTTP OTA endpoint writes that same slot directly. There is no A/B pair, boot table, candidate confirmation, or rollback; recovery is always available through USB Katapult (BareBoot) and, ultimately, BOOTSEL/ROM UF2.

## Requirements

### Requirement: Three-image flash layout
The system SHALL place Katapult stage-0 at flash `0x00000000..0x00008000` (XIP `0x10000000`, 32 KB), the recovery application at flash `0x00008000..0x00048000` (XIP `0x10008000`, 256 KB, built by `agentla-recovery`), and the main application at flash `0x00048000..0x00148000` (XIP `0x10048000`, 1 MB). No boot table or alternate execution slot exists.

#### Scenario: Boot vectoring
- **WHEN** the RP2350 leaves Katapult stage-0 and vectors through the recovery app into the main application
- **THEN** execution begins at the reset handler of the image linked at `0x10048000` and `SCB.VTOR` reports `0x10048000`.

### Requirement: Preferred USER_FS staged OTA
The primary update flow SHALL POST the release `.bin` as raw data to `POST /api/v1/fs/write?root=user&path=/ota/update.bin`, followed by `POST /api/v1/system/reboot`. The recovery app verifies the staged image and installs it only into the main application slot.

#### Scenario: Staging an image into USER_FS
- **WHEN** a client uploads a firmware binary to the USER_FS write endpoint at path `/ota/update.bin`
- **THEN** the file is stored in USER_FS (`0x00248000..0x00400000`) and no raw flash write occurs below `0x48000` or at/above `0x148000`.

#### Scenario: Applying a staged image on reboot
- **WHEN** the system reboots with a staged image present in USER_FS
- **THEN** the recovery app verifies it, writes it into `[0x48000, 0x148000)`, and boots the main application at `0x10048000`.

### Requirement: Legacy streaming OTA REST API
The system SHALL keep `POST /api/v1/system/firmware/ota` (aliased at `/api/v1/firmware/ota`) as a legacy/alternate endpoint that streams a wrapped image straight into the main application slot. The USER_FS staged flow above is preferred.

#### Scenario: Streaming update into the app slot
- **WHEN** an HTTP client streams a valid firmware image to the legacy OTA endpoint
- **THEN** the server erases 4 KB sectors and writes 256-byte pages strictly inside `0x00048000..0x00148000`, returning HTTP 200 after CRC verification.

#### Scenario: Reject oversized image
- **WHEN** an incoming image exceeds 1,048,576 bytes (1 MB) or the 1 MB app slot
- **THEN** the server aborts the stream and returns `HTTP 413 Payload Too Large`.

### Requirement: Image integrity and link-address validation
The legacy stream handler SHALL parse the 64-byte `AGLA_FW\0` header locally (magic, board id, payload size, payload CRC, header CRC with CRC-32 polynomial `0xEDB88320`) and validate the image reset vector before writing. The initial SP must be inside SRAM (`0x20000000..=0x20082000`) and the PC must lie inside the main slot's XIP range starting at `0x10048000`.

#### Scenario: Invalid header, checksum, or vector
- **WHEN** an uploaded image has a corrupted header/CRC or is linked for any address other than `0x10048000`
- **THEN** the server rejects it with `HTTP 400 Bad Request` before modifying the application slot.

### Requirement: BareBoot USB recovery hand-off
After a successful CRC verification the legacy stream handler SHALL write the recovery cookie `0x5984E3FA6CA1589B` to the mailbox derived from the bootloader's initial stack pointer (`read_volatile(0x10000000 as *const u32) as *mut u64`) and call `SCB::sys_reset()`.

#### Scenario: Entering recovery after OTA
- **WHEN** a complete image has been written and verified
- **THEN** the reset makes the BareBoot RP2350 target enter USB Katapult recovery, ready to receive a replacement image; BOOTSEL/ROM UF2 remains the ultimate fallback.

### Requirement: Flash isolation
The OTA writer SHALL NOT erase or program flash below `0x48000` (Katapult stage-0 + recovery app) or at or above `0x148000` (filesystems). `SYSTEM_FS` is flash `0x00148000..0x00248000` (1 MB, 256 blocks) and `USER_FS` is `0x00248000..0x00400000` (1760 KB, 440 blocks). Staged images use the USER_FS LittleFS API, not raw flash writes.

#### Scenario: Formatting the system partition
- **WHEN** a client sends a system partition format request
- **THEN** only `SYSTEM_FS` (`0x00148000..0x00248000`) is reformatted by the WebUI update path; neither OTA flow performs this operation.

### Requirement: Developer Soft BOOTSEL Reboot
The system SHALL expose `POST /api/v1/system/bootsel`, which invokes `embassy_rp::rom_data::reset_to_usb_boot(0, 0)` to enter USB ROM mass-storage mode without a physical button press.

#### Scenario: Executing soft bootsel reboot
- **WHEN** an authorized client sends `POST /api/v1/system/bootsel`
- **THEN** the firmware flushes pending logs and reboots into the USB ROM bootloader.

### Requirement: Debugprobe Hardware Verification
The system SHALL be verifiable on live hardware using an attached Raspberry Pi Debugprobe (`2e8a:000c`) over SWD.

#### Scenario: SWD verification of application vectoring
- **WHEN** the main application executes after leaving the recovery app
- **THEN** the Debugprobe confirms `SCB.VTOR == 0x10048000` and execution at the application's `Reset_Handler`.

### Requirement: Release distribution
The build and release system SHALL produce `agentla-pico2.uf2` containing three segments: Katapult stage-0 at `0x10000000` (32 KB), the recovery application at `0x10008000` (256 KB), and the main firmware at `0x10048000` (1 MB). `webui.tar` is published as a separate release asset and installed at runtime; it is never part of this UF2.

#### Scenario: Flashing the all-in-one UF2 via BOOTSEL
- **WHEN** a user copies `agentla-pico2.uf2` to the `RPI-RP2` USB drive
- **THEN** the RP2350 bootrom writes stage-0, the recovery application, and the main firmware, and the board reboots into AgentLA.

### Requirement: GitHub Actions Tagged Release Workflow
`.github/workflows/release.yml` SHALL trigger on `v*` tags and manual dispatch, build the BareBoot RP2350 target from the `bareBoot` repository, build the in-tree `agentla-recovery` crate (`cargo build --release --target thumbv8m.main-none-eabihf` and objcopy to `build/agentla-recovery.bin`), build `firmware-rs` once with `--no-default-features --features board-pico2`, pass `--recovery` to `tools/package_uf2.py`, and publish `agentla-pico2.uf2`, `agentla-pico2-ota.bin`, `webui.tar`, and `webui.uf2`.
