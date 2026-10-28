## Context

AgentLA is an RP2350-based logic analyzer with three interfaces sharing GPIO0-29:
- **Programmer** (GPIO0-9): Flash programming target MCUs
- **Adapter** (GPIO10-19): Protocol decode (I2C, UART, SPI, CAN)
- **Logic Analyzer** (GPIO20-29): Digital capture

Currently only Adapter and Logic Analyzer exist. The Programmer interface needs to be added as a third interface with exclusive access to GPIO0-9 during programming operations.

Key constraints:
- UART1 TX only on GPIO8 (F0 function)
- UART1 RX only on GPIO9 (F0 function)
- I2C0 SDA on GPIO0 (F2), SCL on GPIO1 (F2)
- PIO0 used for capture, PIO1 available for SWD (Phase 2)
- RP2350 UART bootloader: 1-byte commands at 115200 baud,特定 sync sequence
- STM32 UART bootloader: AN3155 protocol with 0x7F sync, ACK/NACK

## Goals / Non-Goals

**Goals:**
- Add programmer interface with UART flash for STM32, RP2350, NRF, CH32
- Provide exclusive mode: programmer takes over GPIO0-9, releases when done
- Include power enable (GPIO7) and hardware nRST (GPIO6) control
- Self-test loopback on GPIO8↔GPIO9 for hardware verification
- Clean module boundary: `agentla_programmer.c/h` separate from capture/adapter

**Non-Goals:**
- SWD debug (Phase 2 — PIO1 implementation, not in this change)
- Multi-target concurrent programming
- Target power delivery (power enable pin controls external regulator)
- File system for storing firmware images (agent sends raw bytes)

## Decisions

### 1. UART1 hardware peripheral (not PIO) for flash programming
**Choice**: Use hardware UART1 on GPIO8/GPIO9
**Why**: UART1 TX is only available on GPIO8 (F0), RX on GPIO9 (F0). Hardware UART is simpler, more reliable, and frees PIO for capture/SWD. PIO UART possible but adds complexity with no benefit.
**Alternatives considered**: PIO bit-bang UART — rejected because PIO0 is dedicated to capture, PIO1 reserved for SWD.

### 2. Exclusive mode for programmer
**Choice**: Programmer interface has exclusive access to GPIO0-9 during programming
**Why**: Prevents conflicts with adapter mode (I2C on GPIO0/1). Mode switching via `MODE PROG ON` / `MODE PROG OFF` commands. `PROG ON` reconfigures GPIO0-9 as programmer pins, `PROG OFF` releases them.
**Alternatives considered**: Shared mode — rejected because GPIO0/1 conflict between I2C adapter and I2C programmer.

### 3. Power enable via GPIO7 with active-high enable
**Choice**: GPIO7 controls external 3.3V regulator enable pin
**Why**: Allows agent to power target MCU only during programming, saving power. Active-high matches common enable pins on regulators (e.g., AP2112).
**Alternatives considered**: Always-on power — rejected for power safety and thermal reasons.

### 4. nRST control via GPIO6 with open-drain
**Choice**: GPIO6 drives target nRST through open-drain with pull-up
**Why**: nRST is active-low, open-drain allows multiple devices to pull reset. External 10kΩ pull-up to 3.3V on target board.
**Alternatives considered**: Push-pull — rejected because could fight target's internal pull-up/pull-down on nRST.

### 5. STM32 AN3155 protocol implementation
**Choice**: Implement STM32 UART bootloader protocol (AN3155)
**Why**: Most common STM32 bootloader, widely documented. Protocol: sync (0x7F), ACK (0x79), NACK (0x1F), command frames with complement bytes.
**Alternatives considered**: STM32 STLink protocol — rejected because requires SWD, not UART bootloader.

### 6. RP2350 UART bootloader protocol
**Choice**: Implement RP2350 UART bootloader protocol
**Why**: Native RP2350 programming. Protocol: sync (0x7E), ACK, binary load commands.
**Alternatives considered**: USB DFU — rejected because requires USB connection, not available on GPIO header.

## Risks / Trade-offs

- **[Risk] GPIO0/1 I2C conflict** → Mitigated by exclusive mode. User must `PROG OFF` before using adapter I2C.
- **[Risk] UART baud rate mismatch** → Mitigated by auto-baud detection on STM32 (0x7F sync) and fixed 115200 for RP2350.
- **[Risk] Power enable timing** → Mitigated by adding delays after power-on before UART communication.
- **[Trade-off] No SWD in Phase 1** → Accepted. SWD requires PIO1 and is more complex. UART bootloader covers most use cases.
- **[Trade-off] No file system** → Accepted. Agent handles file management, firmware sent as raw bytes over serial.
