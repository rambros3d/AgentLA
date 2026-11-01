# AgentLA

AgentLA is an open-source, all-in-one embedded hardware bring-up lab in a single probe. Powered by the Raspberry Pi RP2040 / RP2350, it combines a multi-target standalone programmer, driverless CMSIS-DAP debug adapter, 10-channel high-speed logic analyzer, signal scope, and multi-protocol bus bridge.

Plug it in over USB, open a browser at `http://agentla.lab` (or `http://192.168.7.1`), and inspect, debug, or flash your target — no proprietary software, drivers, or host toolchain installations required.

---

## Hardware Pin Allocation

AgentLA organizes all 30 GPIO pins into clean, contiguous 10-pin functional domains:

| Domain | GPIO Range | Pin Count | Primary Function |
| :--- | :--- | :--- | :--- |
| **PROGRAMMER** | `GPIO 0 – 9` | 10 pins | SWD, WCH SWIO/SDI, UART ISP, I²C Bootloader, nRST, BOOT0 |
| **ADAPTER** | `GPIO 10 – 19` | 10 pins | I²C, UART, SPI, CAN bus masters & bridges |
| **LOGIC ANALYZER** | `GPIO 20 – 29` | 10 pins | 10-channel high-speed digital capture & protocol decode |

---

## Core Capabilities

### 1. Multi-Target Standalone Programmer (`PROGRAMMER`)
AgentLA contains a fully autonomous on-probe flashing engine capable of detecting, erasing, programming, and verifying target microcontrollers directly via serial CLI (`/dev/ttyACM0` or `/dev/ttyACM1`) or the embedded Web UI, without needing host-side debug tools.

* **Supported Architectures & Families**:
  * **ARM Cortex-M**: 
    * **STMicroelectronics**: STM32F1 (F103 LD/MD/HD/CL/XL), STM32F4 (F411), STM32G0 (G071), STM32L4 (L476), STM32H5 (H503, H52x, H53x, H56x, H57x with 128KB to 2MB Flash).
    * **Puya Semiconductor**: PY32F0 (PY32F030, PY32F003, PY32F002A) with automatic flash size decoding from factory configuration blocks.
    * **Nordic Semiconductor**: nRF52840 (with UICR and hardware mass erase support).
    * **Raspberry Pi**: RP2040, RP2350 (ARM and RISC-V Hazard3 modes).
  * **RISC-V**:
    * **WCH (QingKe RISC-V)**: CH32V003, CH32V203, CH32V307, CH32X035, CH58x, CH59x (CH592 Bluetooth 5.4 MCU).
* **Hardware Protocols**:
  * **ARM SWD (ADIv5)**: 2-wire serial wire debug with continuous-clock turnaround and automatic WAIT/FAULT recovery.
  * **WCH 1-Wire SWIO**: High-speed bi-directional single-pin debug for ultra-low-pin-count chips (CH32V003).
  * **WCH 2-Wire RVSWD / SDI**: Dedicated debug module for CH32V20x/V30x and CH59x MCUs.
  * **UART Bootloader ISP**: Multi-baud scanning (9600 to 460800 baud) and automatic pin polarity swapping.
  * **I²C Bootloader**: Direct hardware EEPROM and MCU bootloader programming.
* **Flashing Features**:
  * Automatic target MCU detection and flash parameter identification (`TARGET DETECT`, `PY32`, `CH32 DETECT`).
  * Direct flash read and memory inspection (`FLASH READ <addr> <len>`).
  * RAM flash algorithm runner (`algo_runner`) executing CMSIS-Pack FLM blobs in target SRAM.
  * Read Protection (RDP) level inspection, RDP locking (`RDPLOCK`), and unprotection (`RDPUNLOCK`).
  * Dedicated hardware `BOOT0` pin driver (`BOOT0 HIGH`, `BOOT0 LOW`) and target reset line (`nRST`).
* **Transport Engines**:
  * **PIO Driver (`PROG DRIVER PIO`)**: Hardware-timed state machine based on Raspberry Pi `debugprobe` (pinned to commit `3fff5b2`), capable of clocking SWD up to 15.6 MHz.
  * **Bitbang Driver (`PROG DRIVER BITBANG`)**: Deterministic SIO bit-twiddling reference driver.

---

### 2. Host Debug Probe & CMSIS-DAP Adapter (`ADAPTER`)
In addition to standalone programming, AgentLA serves as a high-performance host debug probe compatible with standard toolchains.

* **Standard CMSIS-DAP Implementation**:
  * Supports both **CMSIS-DAP v1** (USB HID, zero-driver install on Windows/Linux/macOS) and **CMSIS-DAP v2** (WinUSB bulk endpoints for maximum debug throughput).
  * Fully compatible with standard debuggers: **PyOCD**, **OpenOCD**, **probe-rs**, **Keil MDK**, **IAR EWARM**, **CLion**, and **VS Code (Cortex-Debug)**.
* **Peripheral Bus Adapters & Bridges**:
  * **UART Bridge**: Dual CDC ACM ports for CLI control and high-speed target serial pass-through (300 to 3+ Mbaud, 8N1/8E1/8O1, hardware flow control).
  * **I²C Bus Master**: Interactive bus scanning (`I2C SCAN`), multi-byte reads/writes, sensor evaluation, and register dumps.
  * **SPI Bus Master**: Modes 0–3, configurable clock polarity and phase, multi-chip-select control, and high-speed data streaming.
  * **CAN Bus Interface**: Frame transmission and monitoring on dedicated adapter lines.
  * **GPIO Controller**: Software-configurable inputs, push-pull outputs, internal pull-up/pull-down resistors, and PWM frequency generation.

---

### 3. High-Speed Logic Analyzer (`LOGIC ANALYZER`)
AgentLA features a hardware-accelerated 10-channel digital logic analyzer running on dedicated RP2040/RP2350 PIO state machines.

* **Channels**: 10 simultaneous digital input lines mapped to `GPIO 20 – 29`.
* **Sample Rates**: Configurable sampling from 1 kHz up to 100+ Msps using PIO FIFO DMA transfer to high-speed circular buffers.
* **Hardware Trigger Engine**:
  * Edge triggers: Rising, falling, or both edges on any selected channel.
  * Level & window triggers: High or Low level conditions.
  * Multi-bit pattern triggers with customizable mask matching.
* **Protocol Decoders**:
  * Real-time firmware-level and browser-level decoding.
  * Supported protocols: **UART / Serial**, **I²C**, **SPI**, **PWM / Frequency / Duty Cycle**, and **1-Wire**.
* **Data Export**:
  * Direct capture streaming.
  * Value Change Dump (`.vcd`) export compatible with **Sigrok / PulseView** and GTKWave.

---

### 4. Digital Signal Scope (`SCOPE`)
For signal integrity troubleshooting and bus bring-up, AgentLA provides built-in signal scope capabilities.

* **High-Speed Waveform Capture**:
  * Real-time signal transition recording on critical clock and data lines (`SWD SCOPE`, `SWD SCOPEAB`).
  * Microsecond-level visual inspection of turnaround (Trn) cycles, ACK bit timings, and clock stability.
* **Bus Diagnostics**:
  * Detects floating pins, missing pull-ups, bus contention, and signal skew.
  * Displays ASCII transition diagrams over the serial console and interactive vector waveforms in the Web UI.

---

### 5. Embedded Web Desktop & Virtual Network (`WEB UI`)
AgentLA runs an embedded HTTP server and virtual network stack over USB, delivering a full graphical lab environment without software installation.

* **Virtual Ethernet Interface**:
  * Emulates an RNDIS / CDC-ECM network adapter with built-in DHCP server (`192.168.7.1` / `http://agentla.lab`).
* **WinBox-Style Web Desktop**:
  * **Logic Analyzer Window**: Zoomable waveform viewer, protocol decoding tracks, trigger setup, and VCD export.
  * **Oscilloscope / Scope Window**: Signal transition graphs and bus timing analyzer.
  * **Target Programmer Window**: One-click target detection, memory map view, hex editor, and drag-and-drop `.bin` / `.hex` / `.elf` flashing.
  * **Terminal & Bus Master**: Interactive I²C/SPI/UART consoles for rapid prototyping.

---

### 6. Model Context Protocol (MCP) Server & Agentic Hardware Bring-Up (`MCP`)
AgentLA is purpose-built for the age of agentic engineering. It embeds a native **Model Context Protocol (MCP)** server over its USB virtual Ethernet interface (`http://192.168.7.1/mcp` or `http://agentla.lab/mcp`), enabling AI coding agents (such as Google Antigravity, Claude Desktop, Cursor, and Cline) to directly inspect, interact with, flash, and verify real physical circuits in an autonomous closed loop.

* **Native Embedded HTTP JSON-RPC 2.0**:
  * Direct tool execution via `POST /mcp` with standard JSON-RPC payloads, or query parameters via `GET /mcp?method=cmd&name=<cmd>&args=<args>`.
  * Tool discovery endpoint (`GET /mcp?method=list`) exposing hardware capabilities:
    * `TARGET DETECT` / `SWD` / `CH32` / `PY32`: Autonomous target MCU identification, core status, and flash configuration.
    * `FLASH`: Memory reads, sector erases, FLM pack algorithm execution, and hex writes.
    * `CAPTURE`: Configurable logic analyzer acquisition, signal triggers, and protocol decoding.
    * `I2C_READ` / `I2C_WRITE`: Target I²C sensor and peripheral bus interrogation.
    * `UART_READ` / `UART_WRITE`: Serial telemetry exchange and target bootloader handshaking.
    * `ADC` / `MEASURE`: Voltage measurement and logic rail verification.
    * `PINCLAIM` / `PINRELEASE` / `SAFEDISCOVER`: Hardware pin arbitration and bus safety checks.
* **Asynchronous Jobs & Event Streaming**:
  * Non-blocking queueing for long operations (bulk flash erases, deep logic captures) with asynchronous status polling (`job_status`, `cancel`).
  * Server-Sent Events (`sse_push`) for real-time progress notifications.
* **Hardware Safety Classes**:
  * Built-in safety filters prevent destructive self-tests or diagnostic commands unless explicitly authorized.
* **Agent Integration Example**:
  Add AgentLA to your agent MCP configuration (e.g. `claude_desktop_config.json` or Antigravity MCP settings):
  ```json
  {
    "mcpServers": {
      "agentla": {
        "url": "http://192.168.7.1/mcp"
      }
    }
  }
  ```
  AI models can compile embedded C/Rust firmware, flash the binary through AgentLA, trigger a logic capture, decode UART/I²C packets, and verify test assertions completely autonomously.

---

## Quick Start

### 1. Flashing AgentLA Firmware
1. Hold the **BOOTSEL** button on your Pico / RP2350 board and plug it into USB.
2. Copy `AgentLA.uf2` to the mounted `RPI-RP2` drive.
3. The board reboots automatically into AgentLA mode (USB VID `1209:414d`).

### 2. Serial Console (CLI)
Connect to the primary CDC serial port at 115200 baud:
```bash
picocom -b 115200 /dev/ttyACM0
```
Common commands:
```text
STATUS                     - Probe health and operational mode
TARGET DETECT              - Auto-detect connected ARM/Puya/STM32 MCU
PY32                       - Dedicated Puya PY32 auto-detection & status
CH32 DETECT mcu=ch32v203   - Detect WCH RISC-V target
SWD PINS                   - Display current pin mappings
FLASH READ 0x08000000 64   - Read 64 bytes of target Flash memory
BOOT0 HIGH | BOOT0 LOW     - Control hardware BOOT0 line
BOOTSEL                    - Reboot probe into bootloader for firmware updates
```

### 3. Host Debugging with PyOCD / OpenOCD
AgentLA appears automatically as a CMSIS-DAP probe (`Unique ID: 00000001`):
```bash
# PyOCD
pyocd list
pyocd flash -t py32f030x8 firmware.hex
pyocd gdbserver -t stm32f103c8

# OpenOCD
openocd -f interface/cmsis-dap.cfg -f target/stm32f1x.cfg
```

---

## Attribution

AgentLA builds upon and integrates several open-source libraries, firmware drivers, and tools:

| Project / Library | Primary Role in AgentLA | License | Author / Upstream |
| :--- | :--- | :--- | :--- |
| **[Pico SDK](https://github.com/raspberrypi/pico-sdk)** | Core hardware abstraction layer, clocks, DMA, PIO, and multicore runtime for RP2040 / RP2350 | BSD-3-Clause | Raspberry Pi (Trading) Ltd. |
| **[Raspberry Pi Debugprobe](https://github.com/raspberrypi/debugprobe)** | High-speed PIO-based SWD transfer engine and CMSIS-DAP packet handling | MIT / Apache-2.0 | Raspberry Pi (Trading) Ltd. & ARM Limited |
| **[TinyUSB](https://github.com/hathach/tinyusb)** | USB device stack for CDC serial, CMSIS-DAP HID / WinUSB, and RNDIS / CDC-ECM networking | MIT | Ha Thach & TinyUSB Contributors |
| **[lwIP (Lightweight IP)](https://savannah.nongnu.org/projects/lwip/)** | Embedded TCP/IP stack, sockets, and HTTP server for Web UI and JSON-RPC API backend | BSD-3-Clause | Adam Dunkels & Swedish Institute of Computer Science |
| **[ARM CMSIS-DAP / Core](https://github.com/ARM-software/CMSIS_5)** | Standard Cortex-M debug access port definitions and protocol registers | Apache-2.0 | ARM Limited |
| **[RVSWD_pico](https://github.com/Nicolai-Electronics/RVSWD_pico)** | 2-wire RISC-V debug protocol (`rvswd.pio`) for WCH QingKe V4 cores (CH32V20x/V30x, CH59x) | MIT | Nicolai-Electronics |
| **[PicoRVD](https://github.com/aappleby/picorvd)** | 1-wire SWIO PIO driver and PROGBUF memory access routines for WCH RISC-V targets | MIT | Andrew Appleby |
| **[gusmanb/logicanalyzer](https://github.com/gusmanb/logicanalyzer)** | PIO-accelerated high-speed digital logic capture routines (`capture.pio`) and trigger engine | GPL-3.0 | Bernardo Guasconi |
| **[lrndis](https://github.com/fetisov/lrndis)** | Embedded DHCP (`dhserver.c`), DNS (`dnserver.c`), and RNDIS packet processing | MIT | Sergey Fetisov |
| **[WinBox.js](https://github.com/nextapps-de/winbox)** | Window manager powering the embedded Web Desktop multitool interface | Apache-2.0 | Thomas Wilkerling (Nextapps GmbH) |
