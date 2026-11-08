# Agent Guidelines & Rules for AgentLA

## Interactive Questioning Protocol
- **Always use the `ask_question` tool** whenever asking the user for input, decisions, clarifications, or open questions. Do not ask multiple-choice or decision questions in plain unstructured text.

## Architecture & Code Standards
- **Strict `#![no_std]`**: All firmware and protocol crates must remain `#![no_std]` and zero-heap unless explicitly scoped otherwise.
- **Embassy Async Runtime**: Network, USB, timers, and background tasks run under the Embassy executor with cooperative async I/O.
- **Pin Arbiter Contract**: Never drive or reconfigure pins directly without acquiring ownership from `pin_owner` / `agentla-pin-arbiter`.
- **True A/B Dual-Slot OTA**: Firmware slots (Slot A at `0x10004000`, Slot B at `0x10104000`) and bootloader at `0x10000000` must never overwrite LittleFS partitions (`0x00204000+`).
- **Zero Emojis**: Emojis shall not be used anywhere in this project under any circumstance. All UI text, logs, console output, code, comments, documentation, and tests must remain strictly emoji-free; use standard technical text labels instead.

## UI & Styling Standards
- **No CSS Dot Indicators**: Never use CSS dot indicators or pips below dock app icons, navigation buttons, or tabs (e.g., `.dock-icon-btn.active::after`). Active states must be conveyed through surface styling (subtle background tints or border highlights), never with floating dot indicators.
- **Dock & Viewport Geometry**: The dock must remain anchored directly to the bottom edge (`bottom: 0`, not floating with dead margin), and application viewports must remain full-bleed (`height: calc(100vh - 48px)`) without empty dead-space shelves behind the dock.
- **Launcher Center Alignment**: The application launcher and container must preserve horizontal centering across all zoom levels and display widths using `margin-inline: auto`.

## Hardware Setup & Debugging
- **Debugprobe Available**: The Raspberry Pi Debugprobe (`2e8a:000c`) is wired and available for SWD debugging, register inspection, and fallback hands-free flashing (`probe-rs`) if network OTA or application firmware requires recovery.
