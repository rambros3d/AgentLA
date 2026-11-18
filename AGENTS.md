# Agent Guidelines & Rules for AgentLA

## Interactive Questioning Protocol
- **Always use the `ask_question` tool** whenever asking the user for input, decisions, clarifications, or open questions. Do not ask multiple-choice or decision questions in plain unstructured text.

## Architecture & Code Standards
- **Strict `#![no_std]`**: All firmware and protocol crates must remain `#![no_std]` and zero-heap unless explicitly scoped otherwise.
- **Embassy Async Runtime**: Network, USB, timers, and background tasks run under the Embassy executor with cooperative async I/O.
- **Pin Arbiter Contract**: Never drive or reconfigure pins directly without acquiring ownership from `pin_owner` / `agentla-pin-arbiter`.
- **Three-Image Flash Layout**: Fixed, non-overlapping images on the RP2350 (no A/B pair, boot table, candidate/confirm, or rollback): Katapult stage-0 at flash `0x00000000..0x00008000` (XIP `0x10000000`, 32 KB; jumps to `0x10008000`), recovery app at flash `0x00008000..0x00048000` (XIP `0x10008000`, 256 KB, built by `agentla-recovery`), and the main application slot at flash `0x00048000..0x00148000` (XIP `0x10048000`, 1 MB, built by `firmware-rs`). `SYSTEM_FS` is flash `0x00148000..0x00248000` (1 MB) and `USER_FS` is `0x00248000..0x00400000` (1760 KB, 440 blocks of 4 KB).
- **OTA Writes Are App-Slot Only**: Every OTA write is bounded to `[0x00048000, 0x00148000)`. Never write flash below `0x48000` (Katapult stage-0 + recovery app) or at/above `0x148000` (filesystems). The preferred OTA flow stages the image as a USER_FS file via `POST /api/v1/fs/write?root=user&path=/ota/update.bin` and then reboots; the recovery app verifies and installs it into the app slot. The in-place HTTP `/api/v1/system/firmware/ota` stream remains only as a legacy/alternate path, with the same app-slot bounds.
- **BareBoot RP2350 Recovery Hand-Off**: The legacy OTA path writes `0x5984E3FA6CA1589B` to the mailbox derived from the bootloader's initial stack pointer (`read_volatile(0x10000000 as *const u32) as *mut u64`) and calls `SCB::sys_reset()`, which makes the target enter USB Katapult recovery. BOOTSEL/ROM UF2 remains the ultimate fallback.
- **Never Overwrite LittleFS**: No OTA or packaging operation may write flash below `0x48000` or at/above `0x148000`. `SYSTEM_FS` is never touched by OTA; only the WebUI updater may write it. `USER_FS` is written only through its LittleFS endpoints (for example the staged `/ota/update.bin`), never via raw OTA flash writes.
- **Zero Emojis**: Emojis shall not be used anywhere in this project under any circumstance. All UI text, logs, console output, code, comments, documentation, and tests must remain strictly emoji-free; use standard technical text labels instead.

## UI & Styling Standards
- **No CSS Dot Indicators**: Never use CSS dot indicators or pips below dock app icons, navigation buttons, or tabs (e.g., `.dock-icon-btn.active::after`). Active states must be conveyed through surface styling (subtle background tints or border highlights), never with floating dot indicators.
- **Dock & Viewport Geometry**: The dock must remain anchored directly to the bottom edge (`bottom: 0`, not floating with dead margin), and application viewports must remain full-bleed (`height: calc(100vh - 48px)`) without empty dead-space shelves behind the dock.
- **Launcher Center Alignment**: The application launcher and container must preserve horizontal centering across all zoom levels and display widths using `margin-inline: auto`.

## Hardware Setup & Debugging
- **Debugprobe Available**: The Raspberry Pi Debugprobe (`2e8a:000c`) is wired and available for SWD debugging, register inspection, and fallback hands-free flashing (`probe-rs`) if network OTA or application firmware requires recovery.
