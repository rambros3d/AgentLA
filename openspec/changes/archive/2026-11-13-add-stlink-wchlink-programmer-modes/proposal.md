## Why

AgentLA currently supports ARM Cortex-M debugging through CMSIS-DAP v1 and v2, but vendor-specific desktop ecosystems—notably STMicroelectronics (STM32CubeIDE, STM32CubeProgrammer) and WCH (MounRiver Studio, WCH-Link tools)—require dedicated USB identities and wire protocols (ST-Link V2-1 and WCH RVSWD). Additionally, developers debugging WCH CH32V RISC-V targets currently cannot use AgentLA natively.

By adding runtime-selectable ST-Link and WCH-Link programmer personalities while keeping all instrumentation interfaces (CDC-NCM Ethernet WebUI/MSO, CDC-ACM UART, SocketCAN gs_usb, and i2c-tiny-usb) active, AgentLA becomes a universal multi-mode hardware probe directly compatible with official vendor IDEs and toolchains without exceeding RP2350 USB hardware endpoint limits.

## What Changes

- **ST-Link/V2-1 USB Interface & Protocol**: Add an ST-Link/V2-1 debug endpoint (Vendor Class 0xFF) reporting VID `0x0483` and PID `0x3752` with firmware version `V2J48M35`. Implement ST-Link commands (`F1` version, `F2` SWD/DAP memory and core access, `F3` exit, `F5` mode query).
- **WCH-Link USB Interface & RVSWD Protocol**: Add a WCH-Link debug endpoint (Vendor Class 0xFF) reporting VID `0x1a86` and PID `0x8010`. Implement WCH-Link commands (`0D` version/attach, `08` DMI access, `11` chip info, `01`/`02` flash programming) and bench-verified 2-wire RVSWD protocol framing (double even parity, 4-bit padding, target status ACK) with RISC-V 0.13 DMI/program-buffer execution.
- **Runtime-Selectable Programmer Personality**: Implement a dynamic USB configuration and re-enumeration mechanism that switches the programmer personality between CMSIS-DAP, ST-Link, and WCH-Link upon command.
- **Always-On Instrumentation & Adapters**: Ensure CDC-NCM (Ethernet WebUI/MSO), CDC-ACM (UART telemetry), SocketCAN (`gs_usb`), and I2C (`i2c-tiny-usb`) remain continuously present across all programmer modes.
- **REST & WebUI Controls**: Expose `/api/programmer/mode` to query and switch active programmer personalities, with UI selection integrated into the desktop dashboard.

## Capabilities

### New Capabilities
- `stlink-probe-endpoint`: ST-Link/V2-1 USB vendor interface, version handshake, and SWD/DAP command dispatch for STM32CubeIDE, STM32CubeProgrammer, and OpenOCD.
- `wchlink-probe-endpoint`: WCH-Link USB vendor interface, command dispatch, 2-wire RVSWD framing, and RISC-V 0.13 DMI program-buffer flash programming for CH32V microcontrollers.
- `multi-personality-programmer-switching`: Dynamic USB descriptor assembly and USB re-enumeration manager that swaps programmer personalities while preserving all adapter and instrumentation endpoints.

### Modified Capabilities
- `programmer-mode`: Extend programmer mode selection to support CMSIS-DAP, ST-Link, and WCH-Link personalities with persistence and REST control.

## Impact

- **Crates**:
  - `crates/agentla-programmer`: Add `stlink.rs`, `wch.rs`, `rvdm.rs`; update `rvswd.rs` with double even parity and target status ACK.
  - `firmware-rs`: Update `firmware-rs/src/usb.rs` to support dynamic descriptor generation per personality; update `firmware-rs/src/rest.rs` for mode switching endpoints.
- **Hardware Resources**:
  - RP2350 USB endpoint consumption stays at 10 to 12 endpoints per personality, well within the 16 endpoint hardware limit (8 IN / 8 OUT).
- **Dependencies**: No external crate additions; fully `#![no_std]` compliant using Embassy and existing PIO infrastructure.
