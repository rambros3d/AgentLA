# AgentLA Pin Allocation — RP2350 (QFN-60)

## Overview

All 30 GPIO pins (GPIO0-29) allocated across three interfaces.
Each interface owns a contiguous range, with GPIO 0/1 and 4/5 shared between programmer and adapter.
LED on GPIO25 removed in custom board.

## Allocation Summary

| Range     | GPIO   | Count | Interface |
|-----------|--------|-------|-----------|
| PROGRAMMER| 0-9    | 10    | Flash + Debug |
| ADAPTER   | 10-19  | 10    | I2C + UART + SPI + CAN |
| ANALYZER  | 20-29  | 10    | Logic Analyzer (capture) |

---

## PROGRAMMER (GPIO 0-9)

Flash and debug target MCUs via UART bootloader + SWD.

| GPIO | Name           | Function         | Direction | Notes                          |
|------|----------------|------------------|-----------|--------------------------------|
| 0    | PROG_I2C_SDA   | I2C0 SDA (F2)    | BI        | Target I2C data                |
| 1    | PROG_I2C_SCL   | I2C0 SCL (F2)    | BI        | Target I2C clock               |
| 2    | PROG_BOOT0     | GPIO (SIO)       | OUT       | Dedicated BOOT0 for bootloader recovery (push-pull, `BOOT0 HIGH/LOW`) |
| 3    | —              | GPIO (SIO)       | IN/OUT    | **Reserved / spare** |
| 5    | PROG_SWDIO     | PIO1 (F7)        | BI        | Target SWDIO (SWD)             |
| 6    | PROG_NRESET    | GPIO (SIO)       | OUT       | Target nRST (open-drain)       |
| 7    | —              | GPIO (SIO)       | —         | **NOT USED** — target powered externally (RP2350 GPIO7 max ~12mA insufficient) |
| 8    | PROG_UART_TX   | UART1 TX (F1)    | OUT       | Target RX (UART bootloader)    |
| 9    | PROG_UART_RX   | UART1 RX (F1)    | IN        | Target TX (UART bootloader)    |

**Note:** GPIO 0/1 are shared between programmer I2C and adapter I2C. GPIO 4/5 are shared between SWD (PIO1) and adapter UART. These cannot be used simultaneously.



## SWD Debug Probe

AgentLA supports SWD (Serial Wire Debug) for programming and debugging target MCUs.
SWD is implemented in PIO1 (PIO1 SM0) with modular origin sub-programs.

### Pin Mapping

| AgentLA Pin | GPIO | PIO | Function    | Direction | Target Pin |
|-------------|------|-----|-------------|-----------|------------|
| P_SWCLK     | 4    | 1   | SWCLK       | OUT       | SWCLK      |
| P_SWDIO     | 5    | 1   | SWDIO       | BI        | SWDIO      |
| P_NRST      | 6    | SIO | nRST        | OUT       | nRST       |

### SWD PIO Architecture

- **PIO instance:** PIO1 (SM0)
- **SWCLK:** GPIO 4 (side-set pin 0)
- **SWDIO:** GPIO 5 (IN/OUT/Set pin, pull-up enabled)
- **Default clock:** 1 MHz (max 15.625 MHz at 125 MHz sys_clk)
- **Data shift:** MSB-first via `shift_right=true` in ISR/OSR config
- **ACK extraction:** `(raw_ack >> 29) & 0x07` — PIO shifts bits into ISR[31] and shifts right, so 3-bit ACK lands in bits [31:29]

### SWD Auto-Activation

SWD mode auto-activates on first SWD command. No manual mode switching required.

**Note:** SWD shares GPIO 4-5 with adapter UART. If UART is active, SWD will report a conflict.
GPIO 0-1 are shared between programmer I2C and adapter I2C.

### CLI Commands

```
SWD IDCODE                    # Read target IDCODE
SWD RESET                     # SWD line reset
SWD TEST                      # Connectivity check + target detection
SWD PINS                      # Show pin mapping
SWD READ DP|AP <addr>         # Read DP/AP register
SWD WRITE DP|AP <addr> <val>  # Write DP/AP register
FLASH ERASE <addr> <size>     # Erase flash sectors
FLASH WRITE <addr> <hex_data> # Program flash
FLASH READ <addr> <len>       # Read flash
FLASH VERIFY <addr> <hex>     # Verify flash contents
```

### Supported Targets

| Target      | IDCODE Range     | Flash Algorithm | Status      |
|-------------|------------------|-----------------|-------------|
| STM32F1xx   | 0x0BA00477       | Functional      | ✅ Ready    |
| STM32H5xx   | 0x0BE12477       | Functional (8KB sector, 128-bit write) | ✅ Ready    |
| PY32F0xx    | 0x0BC11477       | Functional (sector erase, 32-bit/page write) | ✅ Ready    |
| RP2350      | 0x00021477 / 0x0BC12477 | Verified (CMSIS-DAP / Probe-rs / Picotool) | ✅ Ready    |
| RP2040      | 0x0BC12477       | Read only       | ⚠️ Planned  |
| STM32F4xx   | 0x0BA00477       | Stub            | ⚠️ Planned  |

### Connections

Connect AgentLA to your target board:
- AgentLA P_SWCLK (GPIO 4) → Target SWCLK
- AgentLA P_SWDIO (GPIO 5) → Target SWDIO
- AgentLA P_NRST (GPIO 6) → Target nRST (optional, for reset)
- GND → Target GND

**Important:** Ensure target is powered externally. GPIO7 provides ~12mA max which is insufficient for most targets (BluePill draws 50-100mA+). Connect 3.3V directly to target VCC.

---

## ADAPTER (GPIO 10-19)

Active bus master: I2C, UART, SPI, CAN.

**Note:** In the current code, adapter I2C uses GPIO 0/1 (shared with programmer I2C) and adapter UART uses GPIO 4/5 (shared with SWD). GPIO 10-19 are reserved for the custom PCB adapter interface.

| GPIO | Name      | Function         | Direction | Notes                     |
|------|-----------|------------------|-----------|---------------------------|
| 10   | A_SCL     | I2C1 SCL (F2)    | BI        | I2C clock (master)        |
| 11   | A_SDA     | I2C1 SDA (F2)    | BI        | I2C data (master)         |
| 12   | A_TX      | UART1 TX (F1)    | OUT       | UART to target            |
| 13   | A_RX      | UART1 RX (F1)    | IN        | UART from target          |
| 14   | A_SCK     | SPI1 SCK (F0)    | OUT       | SPI clock                 |
| 15   | A_MOSI    | SPI1 TX (F0)     | OUT       | SPI master out            |
| 16   | A_MISO    | SPI1 RX (F0)     | IN        | SPI master in             |
| 17   | A_CS      | SPI1 CSn (F0)    | OUT       | SPI chip select           |
| 18   | A_CAN_TX  | PIO1 (F7)        | OUT       | CAN TX → transceiver TXD  |
| 19   | A_CAN_RX  | PIO1 (F7)        | IN        | CAN RX ← transceiver RXD  |

---

## LOGIC ANALYZER (GPIO 20-29)

Passive capture, 10 channels, contiguous. PIO0 IN base = GPIO20, `in pins 10`.

| GPIO | Name      | Function         | Direction | Notes                     |
|------|-----------|------------------|-----------|---------------------------|
| 20   | L_CH0     | PIO0 (F6)        | IN        | Capture channel 0         |
| 21   | L_CH1     | PIO0 (F6)        | IN        | Capture channel 1         |
| 22   | L_CH2     | PIO0 (F6)        | IN        | Capture channel 2         |
| 23   | L_CH3     | PIO0 (F6)        | IN        | Capture channel 3         |
| 24   | L_CH4     | PIO0 (F6)        | IN        | Capture channel 4         |
| 25   | L_CH5     | PIO0 (F6)        | IN        | Capture channel 5         |
| 26   | L_CH6     | PIO0 (F6)        | IN        | Capture ch6 (ADC capable) |
| 27   | L_CH7     | PIO0 (F6)        | IN        | Capture ch7 (ADC capable) |
| 28   | L_CH8     | PIO0 (F6)        | IN        | Capture ch8 (ADC capable) |
| 29   | L_CH9     | PIO0 (F6)        | IN        | Capture ch9 (ADC capable) |

---

## Pin Map

```
GPIO    0   1   2   3   4   5   6   7   8   9
        ├───┤   ·   ·   ├───┤   ├───┤   ├───┤
        I2C             SWD     nRST/PWR UART
        SDA/SCL         SWCLK/  (SIO)    TX/RX
                        SWDIO

GPIO   10  11  12  13  14  15  16  17  18  19
        ├───┤   ├───┤   ├───┤   ├───┤   ├───┤
        I2C     UART    SPI              CAN
        SCL/SDA TX/RX   SCK/MOSI/MISO/CS TX/RX

GPIO   20  21  22  23  24  25  26  27  28  29
        ├───────────────────────────────────────┤
        LOGIC ANALYZER (10 channels)
        CH0  CH1  CH2  CH3  CH4  CH5  CH6  CH7  CH8  CH9
```

## Mode Conflicts

| Combined Mode              | Status     | Notes                                                     |
|----------------------------|------------|-----------------------------------------------------------|
| Analyzer only              | ✓ RUN      | 10 channels on GPIO20-29                                  |
| Adapter only               | ✓ RUN      | I2C+UART+SPI+CAN on GPIO10-19                             |
| Programmer only            | ✓ RUN      | GPIO0-9 for flash/debug                                   |
| Analyzer + Adapter         | ✓ SIMUL    | No GPIO conflict (separate ranges)                        |
| Analyzer + Programmer      | ✓ SIMUL    | No GPIO conflict (separate ranges)                        |
| Adapter + Programmer       | ✗ CONFLICT | GPIO0-1 shared I2C, GPIO4-5 shared (SWD ↔ UART)          |
| All Three                  | ✗ CONFLICT | Adapter + Programmer cannot coexist                       |

### Pin Sharing Details

- **GPIO 0/1:** Programmer I2C and Adapter I2C share these pins (both use I2C)
- **GPIO 4/5:** SWD (PIO1) and Adapter UART share these pins — cannot run SWD and UART simultaneously
- **GPIO 6-9:** Programmer-only (nRST, power, UART) — no adapter conflict

## RP2350 Hardware Constraints

- UART1 TX: GPIO4, 8, 10, 20, 24 (F1) or GPIO6, 10, 22, 26 (F10)
- UART1 RX: GPIO5, 9, 11, 21, 25 (F1) or GPIO7, 11, 23, 27 (F10)
- I2C0: GPIO0-1 (SCL/SDA on F2)
- I2C1: GPIO2-3, 10-11 (SCL/SDA on F2)
- SPI1: GPIO8-11 (RX/CSn/SCK/TX on F0)
- PIO0/1/2: All GPIO0-29 (F6/F7/F8)
- CAN: No hardware peripheral — PIO bit-bang on GPIO18-19

## Dynamic Ownership (v1)

The firmware now reserves `GPIO0–9` for Programmer resources in all modes. Roles on `GPIO10–29` are not assigned at boot; they must be claimed explicitly through the ownership API.

### Ownership Summary

- Flexible claims support Adapter, Capture, bridging, and ADC/Scope-type roles.
- Each successful claim returns a handle. A handle is required by every I2C, UART, and Capture claim-oriented command.
- Capture claims reserve a full contiguous pin span from the lowest to highest selected channel, with the selected subset exposed in metadata.
- Only one capture bank may be active.
- I2C, UART, and Adapter claim handles must carry pin count and function compatibility.
- `PINCLAIM`, `PINRELEASE`, and `PINOWNER` provide basic runtime management.
- `CAPTURECLAIM` and `CAPTURERELEASE` manage span-based capture banks.

### Task 9.2/9.3 Error and Lifecycle Summary

- `PINCLAIM` returns ownership failure when pins are outside `GPIO10–29`, not function-supported, or already owned as drivers.
- `PINOWNER` reports only structured runtime capability.
- Release requests are idempotent; any remaining dependent observers may block release.
- Discovery mode is intended to report only and never mutate functions or claims.
