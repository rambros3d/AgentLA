## Context

Target microcontrollers frequently employ bootloaders such as BareBoot (a Katapult-compatible multi-bus bootloader targeting STM32, CH32V, Puya, and RP microcontrollers) to enable field-upgrades and development firmware deployment over communication buses (UART, I2C, I3C, and CAN).

AgentLA is an embedded hardware bring-up and instrumentation lab running on the RP2350 with Embassy async Rust. It integrates an onboard hardware programmer (SWD, RVSWD, SWIO), transparent UART telemetry, SocketCAN controller (`gs_usb`), Linux I2C (`i2c-tiny-usb`), hardware I3C SDR bridge, and a Web Desktop environment serving LittleFS web applications (`apps/*`).

To simplify workflows, users need a native Web Desktop application in AgentLA that can:
1. Burn BareBoot onto a blank target MCU using AgentLA's hardware programmer.
2. Generate and tune `bareboot.toml` configurations.
3. Perform handsfree firmware updates over UART, I2C, I3C, and CAN.

## Goals / Non-Goals

**Goals:**
- Provide a responsive, self-contained native web application (`apps/bareboot/`) with DaisyUI / Tailwind styling matching AgentLA OS standards.
- Package `apps/packages/bareboot.tar` and register it in `apps/catalog.json`.
- Implement client-side Katapult/BareBoot protocol state machine:
  - Framing: `HEADER` (`[0x01, 0x88]`), CCITT CRC16, and `TRAILER` (`[0x99, 0x03]`).
  - Commands: `CONNECT` (`0x11`), `RX_BLOCK` (`0x12`), `RX_EOF` (`0x13`), `COMPLETE` (`0x15`).
  - In-band handsfree tokens: UART request string (`~ \x1c Request Serial Bootloader!! ~`), I2C/I3C command byte `0xF0`, CAN reboot frame `0x3F0` opcode `0x02`, and hardware pin reset fallback.
- Ship a catalog of pre-compiled BareBoot bootloader images for standard target MCUs (STM32H503 I3C/I2C, STM32F103 BluePill, STM32F401 BlackPill, CH32V203 RISC-V, RP2350) and support drag-and-drop custom `.bin` uploads.
- Add generic, reusable raw stream transfer endpoints in AgentLA firmware (`POST /api/v1/bus/i2c/transfer` and `POST /api/v1/bus/i3c/transfer`) without register pointer constraints.

**Non-Goals:**
- Compiling bootloader binaries on the probe itself (pre-compiled binaries or external toolchain output are used).
- Modifying upstream BareBoot source code (remains read-only reference).
- Replacing standard host command-line utilities (`bareboot-cli`, `probe-rs`).

## Decisions

### Decision 1: Hybrid Architecture (WebUI Protocol Engine + Generic Bus Primitives)
- **Rationale**: Keeping the Katapult protocol state machine in the browser UI (TypeScript/JavaScript) leaves the RP2350 Embassy firmware lean, modular, and unencumbered by domain-specific bootloader logic.
- **Alternatives Considered**:
  - *Firmware-Native Protocol Engine*: Compiling `bareboot-protocol` into `firmware-rs` with high-level `/api/v1/bareboot/*` endpoints. Rejected for long-term maintenance: it tightly couples the AgentLA firmware release cycle to Katapult protocol updates and increases the flash footprint of firmware slots.

### Decision 2: Generic Raw Bus Transfer Endpoints (`/api/v1/bus/i2c/transfer` and `/api/v1/bus/i3c/transfer`)
- **Rationale**: Existing I2C endpoints (`/api/v1/bus/i2c/read` and `write`) assume register-based access (`reg` byte preceding data). Katapult over I2C/I3C requires raw byte streams (`START -> [Addr|W] -> frame -> STOP` and `START -> [Addr|R] -> frame -> STOP`). Adding raw transfer endpoints satisfies BareBoot while providing reusable bus primitives for other future sensor and memory tools.
- **Alternatives Considered**:
  - *Emulating registers via dummy addresses*: Incompatible with Katapult frame parsers which expect `HEADER` immediately after address ACK.

### Decision 3: Bundled Bootloader Catalog with File Upload Dropzone
- **Rationale**: Bootloader binaries are small (~2 KB to 8 KB each). Bundling 5 curated targets into `apps/bareboot/firmwares/` consumes less than 25 KB of LittleFS space while allowing instant zero-friction bootloader bootstrapping. Drag-and-drop file upload allows users with custom pinouts or boards to flash custom builds.

### Decision 4: No CSS Dot Indicators & Strict UI Contract
- **Rationale**: Conforms strictly to AgentLA UI guidelines: no dot indicators under dock icons or tabs, full-bleed viewport (`height: calc(100vh - 48px)`), dock anchored to bottom edge (`bottom: 0`), and zero emojis throughout UI and logs.

## Risks / Trade-offs

- **[Risk] WebSocket / HTTP roundtrip latency during block transfer**:
  - *Mitigation*: Virtual Ethernet (CDC-NCM) has roundtrip latency < 1 ms over USB. For 256-byte blocks on a 128 KB image, transfer completes in under 1 second, with clear live progress feedback.
- **[Risk] Target MCU unresponsive to in-band handsfree tokens**:
  - *Mitigation*: Provide explicit hardware reset fallback: user can trigger hardware nRST line pulse via AgentLA Pin Arbiter to enter the boot window.
- **[Risk] Pin contention between bus adapters and instruments**:
  - *Mitigation*: All bus transactions and SWD programmer actions acquire exclusive pin ownership via `pin_owner` / `agentla-pin-arbiter` and release pins on completion.
