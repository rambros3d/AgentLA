# AgentLA Future Roadmap & Planned Features

This document tracks upcoming and planned features for the **AgentLA RP2350 Firmware** (`firmware-rs`).  
> **Rule**: Once a feature is implemented and verified on hardware, remove it from this file.

---

## 1. Target Debugging & Flashing Expansions

- [x] **Vendor UART Bootloader Recovery Engine (STM32 AN3155)**
  - **Scope**: ST AN3155 USART bootloader (8E1 framing, 0x7F auto-baud sync, 0x92 Readout Unprotect mass erase, extended erase, write memory) with dedicated BOOT0 (GPIO 2) and nRST pin orchestration.
  - **Status**: Completed in `target-debugging-and-flashing-expansions`. REST endpoint `POST /api/v1/programmer/rescue/stm32` available.

- [ ] **WCH CH32 RISC-V 1-Wire SDI / SWIO Debug Engine**
  - **Scope**: Native PIO implementation of WCH proprietary single-wire SDI protocol for CH32V003, CH32V103, CH32V203, and CH32V307.
  - **Capabilities**: Target detection, core halt/run, register/SRAM access, and flash erase/programming via RISC-V ProgBuf execution and debug module registers (`DMCONTROL`, `DMSTATUS`, `DATA0`).
  - **Reference**: `openspec/specs/ch32-swio-pio/spec.md`.

- [x] **Generic Cortex-M Flash Algorithm Runner (CMSIS-Pack FLM / ELF)**
  - **Scope**: Standard ARM CMSIS-Pack flash algorithms (.bin blobs prefixed with 44-byte `AlgoBlobHeader`) stored on LittleFS Partition 1 (`/system/algo/<name>.bin`) via `SYSTEM_FS`. Target SRAM execution harness with stack top pointer and `BKPT 0` halt return code verification.
  - **Status**: Completed in `target-debugging-and-flashing-expansions`. `tools/pack_flm.py` utility available for FLM/ELF packaging.

- [x] **Hardened Connect-under-Reset & Vector Catch Engine**
  - **Scope**: Robust hardware `nRST` assertion, SWD line sync under reset, vector catch (`DEMCR.VC_CORERESET` + `DEMCR.TRCENA`), and software system reset fallback via `AIRCR.SYSRESETREQ`.
  - **Status**: Completed in `target-debugging-and-flashing-expansions`. REST endpoint `POST /api/v1/programmer/reset-catch` available.

- [ ] **Raspberry Pi RP2040 & RP2350 Target Support**
  - **Scope**: SWD programming and rescue debugging for RP2040 and RP2350 targets using Raspberry Pi bootrom vectors and flash memory algorithms.

- [ ] **Espressif ESP32 Series Bootloader Support (ESP32, ESP32-C3, ESP32-C6)**
  - **Scope**: ESP-IDF serial bootloader protocol implementation over target UART pins (`/api/v1/bus/uart`).
  - **Capabilities**: ROM sync (`0x08 0x00`), stub loader injection, SPI flash write, and chip ID detection.

- [ ] **Nordic Semiconductor nRF52 & nRF53 Support**
  - **Scope**: ADIv5 SWD target support with CTRL-AP mass-erase unlock sequence and UICR configuration programming.

- [ ] **Puya PY32F0 Cortex-M0+ Target Support**
  - **Scope**: Native detection, flash unlock, and sector programming for ultra-low-cost PY32F0 targets.

---

## 2. AI Agent (MCP) Hardware Tools

- [ ] **Autonomous Target Circuit Diagnostic (`mcp_target_diagnose`)**
  - **Scope**: One-shot agent tool that inspects target rail voltage, tests pin pull-ups, identifies floating vs. driven lines, probes IDCODE/DPIDR, and determines if the MCU core is running, halted, or in a HardFault loop.

- [ ] **Firmware Vector Table & Symbol Inspector (`mcp_inspect_firmware`)**
  - **Scope**: Reads target vector table, initial SP, reset vector, and option bytes, detecting corrupt vector tables, invalid stack pointers, or memory layout mismatches.

- [ ] **Autonomous Bus Sniffer (`mcp_sniff_bus`)**
  - **Scope**: Agent tool to monitor I2C or SPI pins for a defined duration (e.g. 500 ms) and return a structured list of observed transactions, slave addresses, and NACK events.

---

## 3. Web Desktop, Telemetry & Streaming

- [ ] **In-Browser Web Serial Terminal App (`/apps/terminal`)**
  - **Scope**: An xterm.js terminal client inside the DaisyUI Web Desktop OS (`http://10.10.10.10/apps/terminal`).
  - **Data Path**: Bridges directly to target UART telemetry (GPIO 14 RX / GPIO 8 TX) via WebSocket or SSE, allowing users to interact with target MCU serial consoles without host terminal emulators (`screen`/`picocom`).

- [ ] **Direct Sigrok / Pulseview Protocol Server**
  - **Scope**: Native Sigrok driver backend or continuous framing over CDC-NCM Ethernet/TCP.
  - **Use Case**: Allows Pulseview to connect directly as an external capture device with 1 click.

- [ ] **Continuous Multi-Megabyte DMA Streaming & Real-Time RLE Compression**
  - **Scope**: Upgrade from snapshot ring buffers to continuous PIO+DMA streaming over CDC-NCM Ethernet with real-time Run-Length-Encoding (RLE) on RP2350 Core 1.
  - **Benefit**: Multi-second continuous logic analyzer captures without missing edges or exhausting onboard RAM.

- [ ] **On-Chip & In-Browser Protocol Decoders**
  - **Scope**: Decoders for standard embedded protocols directly within the capture viewer:
    - I2C (Address, Read/Write, Data, ACK/NACK)
    - SPI (Modes 0–3, MOSI/MISO decoded hex/ASCII)
    - UART (Baud auto-detection, framing check)
    - CAN 2.0B & CAN-FD (Standard/Extended ID, DLC, Data, CRC)
    - 1-Wire / Dallas (ROM search, Scratchpad read)
    - DMX512 (Break, MAB, 512-channel slot frames)
    - Modbus RTU (Slave address, function code, CRC16)
    - MIPI I3C SDR (Dynamic Address Assignment, IBI)

---

## 4. Factory Test & Bench Automation (FCT Station)

- [ ] **Dynamic Current & Sleep Profiling with Overcurrent Trip (BakuProbe INA180)**
  - **Scope**: BakuProbe onboard TI INA180 current sense circuit integration.
  - **Protection**: Sub-millisecond short-circuit and overcurrent detection to trip target power immediately upon contact fault.
  - **Telemetry**: Active run current and low-power sleep current draw measurement in microamps.
  - **Reference**: `openspec/changes/autonomous-factory-test-station/specs/`.

- [ ] **Declarative Factory Recipes (`factory_recipe.json`)**
  - **Scope**: Automated execution of multi-step manufacturing test scripts:
    1. Clamp fixture & verify $V_{\text{target}}$ contact
    2. Check quiescent current
    3. Flash firmware via SWD/RVSWD/SWIO
    4. Release reset & assert UART boot banner within timeout
    5. Loopback test onboard CAN-FD / RS-485 transceivers
    6. Record serialization metadata and output PASS/FAIL status
