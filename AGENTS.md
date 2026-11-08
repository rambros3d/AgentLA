# Agent Guidelines & Rules for AgentLA

## Interactive Questioning Protocol
- **Always use the `ask_question` tool** whenever asking the user for input, decisions, clarifications, or open questions. Do not ask multiple-choice or decision questions in plain unstructured text.

## Architecture & Code Standards
- **Strict `#![no_std]`**: All firmware and protocol crates must remain `#![no_std]` and zero-heap unless explicitly scoped otherwise.
- **Embassy Async Runtime**: Network, USB, timers, and background tasks run under the Embassy executor with cooperative async I/O.
- **Pin Arbiter Contract**: Never drive or reconfigure pins directly without acquiring ownership from `pin_owner` / `agentla-pin-arbiter`.
- **True A/B Dual-Slot OTA**: Firmware slots (Slot A at `0x10004000`, Slot B at `0x10130000`) and bootloader at `0x10000000` must never overwrite LittleFS partitions (`0x00270000+`).

## Hardware Setup & Debugging
- **Debugprobe Available**: The Raspberry Pi Debugprobe (`2e8a:000c`) is wired and available for SWD debugging, register inspection, and fallback hands-free flashing (`probe-rs`) if network OTA or application firmware requires recovery.
