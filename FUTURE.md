# AgentLA Future Roadmap & Planned Features

This document tracks upcoming and completed milestones for the **AgentLA Hardware & Firmware System**.
> **Rule**: Once a feature is implemented and verified, REMOVE IT FROM THIS DOCUMENT.

---

## 1. Modularization & Workspace Architecture (Completed)

- [x] **Embassy-Aligned Domain Crates Architecture**
  - **Scope**: Extracted the bare-metal monolithic firmware into 8 independent `#![no_std]` crates under `crates/`:
    - `agentla-bsp`: Board profile traits and external IC drivers (TI INA180, PGA113, TCAN334, TXS0108).
    - `agentla-analyzer`: Logic analyzer capture core, bit-delta RLE block compression, VCD generation, and bus protocol decoders.
    - `agentla-programmer`: Abstract transport traits (`SwdIo`, `SwioIo`, `RvswdIo`), Arm ADIv5, WCH QingKe, CMSIS-DAP v1/v2, FLM flash runner, and feature-gated `rp2350-pio`.
    - `agentla-bridges`: Linux `gs_usb` SocketCAN, `i2c-tiny-usb`, and Lawicel SLCAN over `embedded-hal` and `embedded-can`.
    - `agentla-debug-server`: Transport-agnostic GDB Remote Serial Protocol (RSP) stub and SEGGER RTT circular buffer reader over `embedded-io-async`.
    - `agentla-instruments`: Oscilloscope $V_{pp}$/$V_{rms}$ metrics, zero-crossing frequency, DDS clock divider calculations, and servo PWM timing math.
    - `agentla-pin-arbiter`: In-RAM GPIO pin ownership matrix, all-or-nothing multi-pin claims, and `PinQuiesce` trait.
    - `agentla-mcp`: Embedded Model Context Protocol (MCP) JSON-RPC 2.0 schema and tool dispatcher.
  - **Status**: Completed. 88 host unit tests running in `0.05s` on `x86_64` without hardware. Bare-metal release build for RP2350 verified.

---

## 2. Target Debugging & Flashing Expansions

- [x] **Vendor UART Bootloader Recovery Engine (STM32 AN3155)**
  - **Scope**: ST AN3155 USART bootloader (8E1 framing, 0x7F auto-baud sync, 0x92 Readout Unprotect mass erase, extended erase, write memory) with dedicated BOOT0 and nRST pin orchestration.
  - **Status**: Completed. REST endpoint `POST /api/v1/programmer/rescue/stm32` available.

- [x] **Generic Cortex-M Flash Algorithm Runner (CMSIS-Pack FLM / ELF)**
  - **Scope**: Standard ARM CMSIS-Pack flash algorithms (.bin blobs prefixed with 44-byte `AlgoBlobHeader`) stored on LittleFS Partition 1 (`/system/algo/<name>.bin`) via `SYSTEM_FS`. Target SRAM execution harness with stack top pointer and `BKPT 0` halt return code verification.
  - **Status**: Completed. `tools/pack_flm.py` utility available for FLM/ELF packaging.

- [x] **Hardened Connect-under-Reset & Vector Catch Engine**
  - **Scope**: Robust hardware `nRST` assertion, SWD line sync under reset, vector catch (`DEMCR.VC_CORERESET` + `DEMCR.TRCENA`), and software system reset fallback via `AIRCR.SYSRESETREQ`.
  - **Status**: Completed. REST endpoint `POST /api/v1/programmer/reset-catch` available.

- [x] **Nordic Semiconductor nRF52 & nRF53 Recovery Engine**
  - **Scope**: ADIv5 SWD target support with CoreSight CTRL-AP (AP #1) hardware mass-erase and unprotect sequence.
  - **Status**: Completed in `crates/agentla-programmer/src/target/nrf.rs`.

- [x] **Puya PY32F0 Cortex-M0+ Target Detection & Support**
  - **Scope**: Native detection, flash unlock, and sector identification for ultra-low-cost PY32F0 targets.
  - **Status**: Completed in `crates/agentla-programmer/src/target/targets.rs`.

- [x] **WCH CH32 RISC-V 1-Wire SDI / SWIO & 2-Wire RVSWD Hardware Verification**
  - **Scope**: Native PIO implementation of WCH proprietary single-wire SDI protocol for CH32V003, and 2-wire RVSWD for CH32V203/CH592.
  - **Capabilities**: Target detection, core halt/run, register/SRAM access, and flash inspect via debug module registers (`DMCONTROL`, `DMSTATUS`, `DATA0`, `CFGR`, `SHDWCFGR`).
  - **Hardware Verified**: Live verified on physical hardware across Port 1 (STM32F103 SWD), Port 2 (CH32V203 2-wire RVSWD), and Port 3 (CH32V003 1-wire SWIO) simultaneously via REST (`/api/v1/programmer/target?port=...`) and MCP tools (`probe_target`, `target_diagnose`). Active push-pull transmission with 96 ns active pull-up acceleration eliminates the need for external 1k pull-up resistors on unbuffered SWIO lines.

- [ ] **Raspberry Pi RP2040 & RP2350 Target Bootrom Flashing**
  - **Scope**: SWD flash programming and rescue debugging for RP2040 and RP2350 targets using Raspberry Pi bootrom vectors and flash memory algorithms.

- [ ] **Espressif ESP32 Series Bootloader Support (ESP32, ESP32-C3, ESP32-C6)**
  - **Scope**: ESP-IDF serial bootloader protocol implementation over target UART pins (`/api/v1/bus/uart`).
  - **Capabilities**: ROM sync (`0x08 0x00`), stub loader injection, SPI flash write, and chip ID detection.

---

## 3. AI Agent (MCP) Hardware Tools (Completed)

- [x] **Autonomous Target Circuit Diagnostic (`mcp_target_diagnose`)**
  - **Scope**: One-shot agent tool that inspects target rail voltage, tests pin pull-ups, identifies floating vs. driven lines, probes IDCODE/DPIDR, and determines if the MCU core is running, halted, or in a HardFault loop with automated root-cause diagnosis (Divide-by-zero, unaligned access, invalid Thumb state, precise BusFault, double-fault lockup).
  - **Status**: Completed in `agentla-mcp` and verified in `test/test_mcp_target_diagnose.py`.

- [x] **Firmware Vector Table & Symbol Inspector (`mcp_inspect_firmware`)**
  - **Scope**: Reads target vector table, initial SP, reset vector, and option bytes, detecting corrupt vector tables, invalid stack pointers, or memory layout mismatches.
  - **Status**: Completed in `agentla-mcp` and verified in `test/test_mcp_inspect_firmware.py`.

- [x] **Autonomous Bus Sniffer (`mcp_sniff_bus`)**
  - **Scope**: Agent tool to monitor I2C or SPI pins for a defined duration (e.g. 500 ms) and return a structured list of observed transactions, slave addresses, and NACK events.
  - **Status**: Completed in `agentla-mcp` and verified in `test/test_mcp_sniff_bus.py`.

---

## 4. Web Desktop, Telemetry & Streaming

- [ ] **Continuous Multi-Megabyte DMA Streaming & Real-Time RLE Compression**
  - **Scope**: Upgrade from snapshot ring buffers to continuous PIO+DMA streaming over CDC-NCM Ethernet with real-time Run-Length-Encoding (RLE) on RP2350 Core 1.
  - **Benefit**: Multi-second continuous logic analyzer captures without missing edges or exhausting onboard RAM.

- [ ] **Direct Sigrok / Pulseview Protocol Server**
  - **Scope**: Native Sigrok driver backend or continuous framing over CDC-NCM Ethernet/TCP.
  - **Use Case**: Allows Pulseview to connect directly as an external capture device with 1 click.

- [ ] **In-Browser Web Serial Terminal App (`/apps/terminal`)**
  - **Scope**: An xterm.js terminal client inside the DaisyUI Web Desktop OS (`http://10.10.10.10/apps/terminal`).
  - **Data Path**: Bridges directly to target UART telemetry via WebSocket or SSE, allowing users to interact with target MCU serial consoles without host terminal emulators (`screen`/`picocom`).

---

## 5. Factory Test & Bench Automation (FCT Station)

- [ ] **Autonomous Factory Test Station (`autonomous-factory-test-station`)**
  - **Scope**: Production programming and functional verification fixture:
    1. Continuous $V_{\text{target}}$ contact sensing with 150 ms debounce.
    2. Sub-millisecond short-circuit and overcurrent trip protection via onboard TI INA180 current sense.
    3. Premature unclamp safe High-Z pin release.
    4. Declarative `factory_recipe.json` test runner with dynamic serial number / MAC address patching.
    5. Automated UART boot banner verification, TCAN334 CAN-FD echo, and RS-485 ping.
    6. Full-screen production WebUI with 2-meter readable PASS/FAIL status cards and yield counters.
  - **Reference**: `openspec/changes/autonomous-factory-test-station/`.

---

## 6. BakuProbe Hardware Bring-Up (Custom PCB)

- [ ] **BakuProbe RP2350B QFN-80 Board Bring-Up**
  - **Scope**: Hardware driver activation for reference BakuProbe PCB:
    - Pod 4 & 5: 8-channel bidirectional logic analyzer + 8-channel MSO oscilloscope frontend with CH444G 4:1 analog attenuators.
    - 32-bit shift register bus (`SPI0`) for transceiver OE and attenuator gain switching.
    - Per-pin chained SK6805-EC15 / WS2812B micro-RGB LEDs reflecting real-time pin roles.
    - Front-panel tactile emergency stop trip button (`EMERGENCY_BTN`).
