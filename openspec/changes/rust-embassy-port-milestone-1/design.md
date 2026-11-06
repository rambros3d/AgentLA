## Context

AgentLA is transitioning to an asynchronous Rust architecture powered by Embassy on the Raspberry Pi RP2350 (ARM Cortex-M33). The current C implementation operates a complex cooperative event loop running lwIP, TinyUSB, LittleFS, and hardware drivers across dual cores.

Milestone 1 establishes the clean-slate foundation: setting up a separate git worktree (`../AgentLA-rust`) on branch `rust`, establishing the `embassy-rp` runtime, building the composite USB stack, and bringing up the asynchronous network and web server.

## Goals / Non-Goals

**Goals:**
- Provide a clean, isolated development environment on git branch `rust` via git worktrees without mutating the existing C codebase or CMake build.
- Implement a fully asynchronous RP2350 runtime using `embassy-rp` (`thumbv8m.main-none-eabihf`).
- Configure an `embassy-usb` composite device with:
  - Dual CDC-ACM (Port 0: CLI / REPL, Port 1: target UART bridge).
  - CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (Vendor Bulk) interface scaffolding.
  - CDC-NCM (Network Control Model) virtual Ethernet interface.
- Run `embassy-net` with static IP `10.10.10.10/24`, an async DHCP server task, and DNS captive portal.
- Run `picoserve` async HTTP server serving static files from an on-flash LittleFS partition (`littlefs2`), JSON REST API, and SSE telemetry stream.

**Non-Goals:**
- Porting the full target flashing engine (CH32, STM32, PY32) — reserved for Milestone 3.
- Implementing the 10-channel PIO logic analyzer and protocol decoders — reserved for Milestone 2.
- Support for RP2040 in Milestone 1 (RP2350 is the sole focus; RP2040 compatibility will follow via Cargo feature flags later).

## Decisions

### 1. Separate Git Worktree on Branch `rust`
- **Choice**: `git worktree add -b rust ../AgentLA-rust dev`
- **Rationale**: Keeps the C build system, Python verification suite, and existing OpenSpec changes intact and clean, avoiding churn in `.gitignore`, `CMakeLists.txt`, and submodules.
- **Alternatives Considered**: Subdirectory `firmware-rs/` in main tree (clutters main git status and history before stabilization).

### 2. RP2350 ARM Cortex-M33 Target (`thumbv8m.main-none-eabihf`)
- **Choice**: Target RP2350 Cortex-M33 first using `embassy-rp` with `rp2350` feature.
- **Rationale**: RP2350 provides 520 KB of SRAM (vs 264 KB on RP2040) and 16 bidirectional USB endpoints (EP0..EP15). This eliminates endpoint starvation and memory pressure while bringing up complex composite USB + TCP/IP + HTTP stacks.
- **Alternatives Considered**: RP2040 first (more constrained endpoints and RAM).

### 3. Composite USB via `embassy-usb`
- **Choice**: Single device descriptor with multiple interface descriptors:
  - Interface 0 & 1: CDC-ACM 0 (CLI / Control)
  - Interface 2 & 3: CDC-ACM 1 (UART Target Bridge)
  - Interface 4: CMSIS-DAP v1 (HID)
  - Interface 5: CMSIS-DAP v2 (Vendor Bulk endpoints)
  - Interface 6 & 7: CDC-NCM (Communications & Data)
- **Rationale**: `embassy-usb` provides an asynchronous builder pattern that naturally handles composite endpoint allocation.
- **Alternatives Considered**: TinyUSB Rust bindings (defeats the memory safety and pure async benefits of Embassy).

### 4. Asynchronous Web Server: `picoserve` + `embassy-net` + `littlefs2`
- **Choice**: Use `picoserve` on top of `embassy-net` TCP listeners, reading web assets from LittleFS via `littlefs2`.
- **Rationale**: `picoserve` is built explicitly for Embassy and `no_std`, using compile-time routing and zero dynamic allocations. `littlefs2` preserves binary compatibility with existing AgentLA LittleFS web asset images.
- **Alternatives Considered**: Custom raw TCP socket parsing (high maintenance); `microdot` (less feature-complete than `picoserve`).

## Risks / Trade-offs

- **[Risk] CDC-NCM Compatibility on Legacy Windows**: Windows 10 does not support CDC-NCM natively without third-party drivers (Windows 11 and Linux/macOS work out of the box).
  - *Mitigation*: Windows 11 and modern Linux/macOS are the primary targets for developer workstations; RNDIS fallback driver can be layered in if legacy Windows 10 support is strictly required.
- **[Risk] Flash partition alignment**: Rust LittleFS driver must write to the exact flash offset reserved for web assets and configurations.
  - *Mitigation*: Define a shared memory layout constant (e.g. `FLASH_FS_OFFSET = 1024 * 1024`, 1MB offset) matching the C board settings.
