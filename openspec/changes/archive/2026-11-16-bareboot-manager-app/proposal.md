## Why

Target microcontrollers frequently run bootloaders such as BareBoot (a Katapult-compatible multi-bus bootloader) to enable field-upgrades and development firmware deployment over communication buses (UART, I2C, I3C, and CAN). Currently, users must juggle separate command-line host utilities (like `bareboot-cli` or `katapult/flashtool.py`) and external hardware programmers to bootstrap chips and deploy firmware updates.

Providing a native Web Desktop application inside AgentLA (`apps/bareboot`) allows engineers to visually detect target microcontrollers, burn BareBoot onto fresh chips via AgentLA's onboard SWD/RVSWD programmer, generate and tune `bareboot.toml` configurations, and stream application firmware updates across UART, CAN, I2C, or I3C directly from the browser without installing host-side toolchains.

## What Changes

- Introduce the `bareboot` native Web Desktop application in `apps/bareboot/` (packaged as `apps/packages/bareboot.tar` and indexed in `apps/catalog.json`).
- Implement the client-side Katapult/BareBoot protocol state machine in JavaScript/TypeScript supporting:
  - Handsfree bootloader entry tokens: UART serial request string, I2C/I3C command byte `0xF0`, CAN reboot frame `0x3F0`, and hardware reset fallback.
  - Katapult framing with CCITT CRC16, frame synchronization, and verification.
  - Handshake session management (`CONNECT`, `RX_BLOCK`, `RX_EOF`, `COMPLETE`).
  - Block streaming engine with live transfer rate and progress tracking.
- Provide a bundled catalog of pre-compiled BareBoot bootloader images for standard target MCUs (STM32H503 I3C/I2C, STM32F103 BluePill, STM32F401 BlackPill, CH32V203 RISC-V, and RP2350) alongside custom binary drag-and-drop upload.
- Provide a visual `bareboot.toml` configuration profile builder.
- Add generic raw stream transfer REST endpoints to AgentLA firmware (`POST /api/v1/bus/i2c/transfer` and `POST /api/v1/bus/i3c/transfer`) to execute raw multi-byte bus transactions without register pointer semantics.

## Capabilities

### New Capabilities
- `bareboot-manager-application`: Native Web Desktop app (`apps/bareboot/`) providing a comprehensive workbench for bootstrapping BareBoot via SWD, configuring `bareboot.toml`, and updating target application firmware over UART, I2C, I3C, and CAN.
- `bus-raw-transfers`: Generic raw transaction endpoints (`/api/v1/bus/i2c/transfer`, `/api/v1/bus/i3c/transfer`) in AgentLA firmware executing raw multi-byte write-then-read operations without register offset semantics.

### Modified Capabilities
- `bareboot-i3c-bootloader-flashing`: Broaden scope beyond STM32H503 I3C bootloader flashing to include multi-target catalog discovery and automated hardware flashing verification across all supported BareBoot target families.

## Impact

- `apps/catalog.json`: Added entry for `bareboot` app package.
- `apps/bareboot/`: New application bundle (`app.json`, `icon.svg`, `index.html`, `catalog.json`, `firmwares/*.bin`).
- `firmware-rs/src/rest.rs` & `firmware-rs/src/web.rs`: New route services for raw I2C and I3C transactions.
- Zero breaking changes to existing REST endpoints or Embassy kernel structures.
