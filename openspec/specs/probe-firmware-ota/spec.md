# probe-firmware-ota Specification

## Purpose
Provides an in-service single-slot firmware OTA update subsystem for the AgentLA probe. An uploaded image is streamed straight into the one application slot, verified by CRC-32, and handed off to the BareBoot RP2350 USB recovery target. There is no A/B pair, boot table, candidate confirmation, or rollback; recovery is always available through USB Katapult (BareBoot) and, ultimately, BOOTSEL/ROM UF2.

## Requirements

### Requirement: Single application slot
The system SHALL execute application firmware from flash `0x00008000..0x00140000` (XIP `0x10008000`, 1248 KB). The BareBoot RP2350 recovery bootloader occupies flash `0x00000000..0x00008000` (XIP `0x10000000`, 32 KB). No boot table or alternate execution slot exists.

#### Scenario: Boot vectoring
- **WHEN** the RP2350 leaves BareBoot and vectors into the application
- **THEN** execution begins at the reset handler of the image linked at `0x10008000` and `SCB.VTOR` reports `0x10008000`.

### Requirement: Streaming OTA REST API
The system SHALL expose `POST /api/v1/system/firmware/ota` (aliased at `/api/v1/firmware/ota`) that accepts chunked uploads and writes the payload only into the single application slot.

#### Scenario: Streaming update into the app slot
- **WHEN** an HTTP client streams a valid firmware image to the OTA endpoint
- **THEN** the server erases 4 KB sectors and writes 256-byte pages strictly inside `0x00008000..0x00140000`, returning HTTP 200 after CRC verification.

#### Scenario: Reject oversized image
- **WHEN** an incoming image exceeds 1,228,800 bytes (1.2 MB) or the 1248 KB slot
- **THEN** the server aborts the stream and returns `HTTP 413 Payload Too Large`.

### Requirement: Image integrity and link-address validation
The system SHALL parse the 64-byte `AGLA_FW\0` header locally (magic, board id, payload size, payload CRC, header CRC with CRC-32 polynomial `0xEDB88320`) and validate the image reset vector before writing. The initial SP must be inside SRAM (`0x20000000..=0x20082000`) and the PC must lie inside the single slot's XIP range starting at `0x10008000`.

#### Scenario: Invalid header, checksum, or vector
- **WHEN** an uploaded image has a corrupted header/CRC or is linked for any address other than `0x10008000`
- **THEN** the server rejects it with `HTTP 400 Bad Request` before modifying the application slot.

### Requirement: BareBoot USB recovery hand-off
After a successful CRC verification the system SHALL write the recovery cookie `0x5984E3FA6CA1589B` to the mailbox derived from the bootloader's initial stack pointer (`read_volatile(0x10000000 as *const u32) as *mut u64`) and call `SCB::sys_reset()`.

#### Scenario: Entering recovery after OTA
- **WHEN** a complete image has been written and verified
- **THEN** the reset makes the BareBoot RP2350 target enter USB Katapult recovery, ready to receive a replacement image; BOOTSEL/ROM UF2 remains the ultimate fallback.

### Requirement: LittleFS isolation
The OTA writer SHALL NOT erase or program flash at or above `0x140000`. `SYSTEM_FS` is flash `0x140000..0x240000` (1 MB) and `USER_FS` is `0x240000..0x400000` (1792 KB).

#### Scenario: Formatting the system partition
- **WHEN** a client sends `POST /api/v1/system/fs/format` with `{"partition": "system"}`
- **THEN** the firmware erases `SYSTEM_FS` (`0x140000..0x240000`) and reinitializes its LittleFS structure; an OTA upload never performs this operation.

### Requirement: Developer Soft BOOTSEL Reboot
The system SHALL expose `POST /api/v1/system/bootsel`, which invokes `embassy_rp::rom_data::reset_to_usb_boot(0, 0)` to enter USB ROM mass-storage mode without a physical button press.

#### Scenario: Executing soft bootsel reboot
- **WHEN** an authorized client sends `POST /api/v1/system/bootsel`
- **THEN** the firmware flushes pending logs and reboots into the USB ROM bootloader.

### Requirement: Debugprobe Hardware Verification
The system SHALL be verifiable on live hardware using an attached Raspberry Pi Debugprobe (`2e8a:000c`) over SWD.

#### Scenario: SWD verification of application vectoring
- **WHEN** the application executes after leaving BareBoot
- **THEN** the Debugprobe confirms `SCB.VTOR == 0x10008000` and execution at the application's `Reset_Handler`.

### Requirement: Release distribution
The build and release system SHALL produce `agentla-pico2.uf2` containing exactly two segments: the BareBoot RP2350 recovery bootloader at `0x10000000` (32 KB) and the single-slot firmware at `0x10008000`. `webui.tar` is published as a separate release asset and installed at runtime; it is never part of this UF2.

#### Scenario: Flashing the all-in-one UF2 via BOOTSEL
- **WHEN** a user copies `agentla-pico2.uf2` to the `RPI-RP2` USB drive
- **THEN** the RP2350 bootrom writes the bootloader and firmware, and the board reboots into AgentLA with BareBoot recovery installed.

### Requirement: GitHub Actions Tagged Release Workflow
`.github/workflows/release.yml` SHALL trigger on `v*` tags and manual dispatch, build the BareBoot RP2350 target from the `bareBoot` repository, build `firmware-rs` once with `--no-default-features --features board-pico2`, and publish `agentla-pico2.uf2`, `agentla-pico2-ota.bin`, `webui.tar`, and `webui.uf2`.
