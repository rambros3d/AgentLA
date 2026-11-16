## Context

AgentLA is an RP2350-based high-performance logic analyzer, mixed-signal oscilloscope (MSO), and hardware debugger running `#![no_std]` Rust with the Embassy async executor. While CMSIS-DAP v1 and v2 are currently supported for generic ARM debugging, primary target ecosystems rely on dedicated vendor identities and protocols:
- **STMicroelectronics**: Tools like STM32CubeIDE (`ST-LINK_gdbserver`) and STM32CubeProgrammer mandate genuine ST-Link USB identifiers (`0483:3752`) and proprietary command protocols (`F1`, `F2`).
- **WCH (WinChipHead)**: CH32V series RISC-V microcontrollers utilize a proprietary 2-wire serial protocol (RVSWD) on PA13/PA14 rather than standard ARM SWD or standard JTAG. Tools like MounRiver Studio, `wlink`, and WCH OpenOCD mandate the WCH-Link USB identity (`1a86:8010`).

The upstream open-source firmware `hybridlink` ([`ref/hybridlink`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink)) successfully implements this multi-personality model in C without vendor libraries. This design ports and adapts these proven protocols into `AgentLA-rust`.

## Goals / Non-Goals

**Goals:**
- Provide a runtime-selectable programmer subsystem supporting three distinct personalities:
  1. CMSIS-DAP v1 / v2 (`0x1209:0x414D`).
  2. ST-Link/V2-1 (`0x0483:0x3752`) with version handshake `V2J48M35`.
  3. WCH-Link (`0x1a86:0x8010`) with firmware version `2.12`.
- Keep all non-programmer features always active and accessible across every personality:
  - CDC-NCM Ethernet (WebUI dashboard, MSO, REST, WebSockets).
  - CDC-ACM UART (Target telemetry and console).
  - SocketCAN `gs_usb`.
  - I2C `i2c-tiny-usb`.
- Implement bench-verified 2-wire RVSWD framing for CH32V targets with double-even-parity and target status ACK (`0111`).
- Support zero-blob flash programming on CH32V targets using RISC-V 0.13 Debug Module abstract commands and program-buffer streaming.
- Expose runtime mode selection via WebUI and REST (`/api/programmer/mode`).

**Non-Goals:**
- Supporting 1-wire SWIO (CH32V003) in the initial milestone (reserved for future enhancement).
- Supporting legacy ST SWIM or USB Mass Storage drag-and-drop programming.
- Supporting JTAG mode in ST-Link (AgentLA routes SWD pins for probe operations).

## Decisions

### Decision 1: Runtime Dynamic USB Re-enumeration over Fixed Composite

- **Choice**: When switching programmer personalities, the device tears down the USB stack, rebuilds descriptors with the target vendor VID/PID, and reconnects D+, rather than exposing all vendor endpoints simultaneously under AgentLA's default VID/PID.
- **Rationale**:
  - Official vendor tools (STM32CubeIDE, CubeProgrammer, MounRiver Studio) perform strict root-level VID/PID filtering.
  - Exposing all endpoints simultaneously would consume 19 physical endpoints, exceeding the RP2350 hardware ceiling of 16 endpoints (8 IN, 8 OUT).
  - Re-enumeration keeps total endpoint consumption at 10 to 12 endpoints per personality.
- **Alternatives Considered**:
  - *All-in-one mega-composite*: Discarded due to hardware endpoint exhaustion and rejection by vendor desktop tools.

### Decision 2: Endpoint Allocation Strategy

- **Choice**: Reserve physical endpoints 0x01/0x81 and 0x02/0x82 for the active programmer personality, and allocate endpoints 0x03/0x83 through 0x06/0x86 to the always-on base services:
  - CDC-ACM (UART): 1 interrupt IN (0x83), 1 bulk OUT (0x03), 1 bulk IN (0x84)
  - CDC-NCM (Ethernet): 1 interrupt IN (0x85), 1 bulk OUT (0x04), 1 bulk IN (0x86)
  - SocketCAN (`gs_usb`): 1 bulk OUT (0x05), 1 bulk IN (0x87)
  - `i2c-tiny-usb`: Control transfers on EP0
- **Rationale**: WCH host tools hard-code EP 0x01/0x81 (commands) and EP 0x02/0x82 (data), and ST-Link tools hard-code EP 0x01/0x81. Giving them the lowest endpoint indices ensures seamless host tool compatibility.

### Decision 3: Zero-Blob RISC-V Flash Programming via Program Buffer

- **Choice**: Implement `rvdm_mem_stream32` using RISC-V 0.13 Debug Module abstract commands with `postexec` on `DM_PROGBUF0` rather than uploading and executing closed-source RAM algorithms.
- **Rationale**:
  - Avoids licensing and binary distribution issues with proprietary vendor flash algorithms.
  - Streaming words via `INSN_SW_S1_S0` and hardware `WRBSY` polling achieves maximum bus throughput directly over RVSWD.

## Risks / Trade-offs

- **[Risk] Host network interface drop during USB re-enumeration**
  - *Mitigation*: The CDC-NCM Ethernet driver re-establishes link within 500 ms; WebUI front-end uses automatic reconnect with exponential backoff on WebSockets.
- **[Risk] Operating system driver caching on Windows**
  - *Mitigation*: Provide standard Microsoft OS 2.0 WinUSB descriptors for CMSIS-DAP v2 and WCH-Link vendor interfaces so Windows does not require driver installation.
- **[Risk] RVSWD half-duplex turnaround race conditions**
  - *Mitigation*: RP2350 PIO state machine handles precise clock-to-data turnaround transitions without software jitter.
