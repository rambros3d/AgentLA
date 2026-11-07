## 1. Boot Table Metadata & Geometry Layout

- [x] 1.1 Shift partition offsets in `firmware-rs/src/storage.rs`: `PROFILE_WINDOW_START` to `0x00260000`, `SYSTEM_FS` to `0x00270000`, and `USER_FS` to `0x002F0000`
- [x] 1.2 Create `#![no_std]` crate `crates/agentla-boot-proto` defining `BootTable`, `BootState`, `SlotId`, image headers (`AGLA_FW`), and CRC-32 integrity verification
- [x] 1.3 Implement atomic boot metadata read, update, and commit routines with ping-pong sector redundancy in flash (`0x00002000` - `0x00004000`)

## 2. Linker Scripts & Slot Memory Layouts

- [x] 2.1 Update linker scripts (`boards/pico2/memory.x` and `boards/bakuprobe/memory.x`) to define Slot A (`0x10004000`, 1.2 MB) and Slot B (`0x10130000`, 1.2 MB) execution windows
- [x] 2.2 Configure build profiles or cargo features for targeting Slot A and Slot B binaries
- [x] 2.3 Implement position-independent image header generation script with magic `AGLA_FW`, target board ID, and payload length

## 3. Minimal Boot Switcher

- [x] 3.1 Create `#![no_std]` binary crate `crates/agentla-bootloader` (< 4 KB) placed at flash offset `0x00000000` (`0x10000000` XIP)
- [x] 3.2 Implement boot metadata evaluation: candidate attempt decrement, watchdog arming (15 seconds), and automatic fallback to previous active slot
- [x] 3.3 Set Cortex-M33 VTOR to target slot (`0x10004000` or `0x10130000`), load initial SP and branch to Reset_Handler

## 4. In-Service HTTP Streaming OTA & REST API

- [x] 4.1 Implement `POST /api/v1/system/firmware/ota` chunked streaming endpoint in `firmware-rs/src/web.rs` and `rest.rs`
- [x] 4.2 Stream incoming chunks directly into the inactive slot sectors using flash page programming with zero intermediate heap/RAM buffers
- [x] 4.3 Validate `AGLA_FW` header magic, target board ID, slot destination bounds, and SHA-256 / CRC-32 checksums
- [x] 4.4 Implement `GET /api/v1/system/firmware/status` to report active slot, candidate slot, and firmware versions
- [x] 4.5 Implement `POST /api/v1/system/firmware/rollback` to trigger intentional immediate rollback to the alternate slot
- [x] 4.6 Decommission runtime BOOTSEL endpoint (`GET /api/v1/system/bootsel`) in `firmware-rs/src/web.rs` and MCP tool (`system_reboot_bootsel`) in `firmware-rs/src/mcp.rs`

## 5. Watchdog Handshake & Self-Test Confirmation

- [x] 5.1 Initialize RP2350 hardware watchdog during early boot in `firmware-rs` with candidate 15-second timeout
- [x] 5.2 Implement `ota_confirm()` called after USB CDC-NCM and network stack initialization, committing candidate slot as permanently active
- [x] 5.3 Ensure watchdog is transitioned to standard steady-state pet interval once confirmed

## 6. Testing & Live Hardware Verification via Attached Debugprobe

- [x] 6.1 Unit test `BootTable` encoding/decoding, CRC verification, and state transition logic on `x86_64`
- [x] 6.2 Verify build of boot switcher (`crates/agentla-bootloader`), Slot A, and Slot B firmware images
- [x] 6.3 Flash initial boot switcher + Slot A to hardware via attached Raspberry Pi Debugprobe (`probe-rs download --chip RP2350`)
- [x] 6.4 Validate bootloader VTOR setting and core vectoring using Debugprobe SWD inspection
- [x] 6.5 Stream Slot B update live over CDC-NCM Ethernet via `POST /api/v1/system/firmware/ota`
- [x] 6.6 Verify candidate boot into Slot B, automatic `ota_confirm()`, and LittleFS partition preservation at `0x00270000` (`SYSTEM_FS`) and `0x002F0000` (`USER_FS`)
- [x] 6.7 Verify watchdog rollback mechanism via `POST /api/v1/system/firmware/rollback` or unconfirmed reboot reverting cleanly to Slot A
