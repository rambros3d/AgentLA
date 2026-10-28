## Why

AgentLA currently supports two interfaces: Logic Analyzer (GPIO20-29) and Adapter (GPIO10-19). Users need a third interface for programming target MCUs (flashing firmware) directly from the device, eliminating the need for separate programmers. The RP2350 has available GPIO0-9 with UART1 TX on GPIO8 and GPIO0+1 for I2C0, enabling UART-based bootloader flashing for STM32, RP2350, NRF, and CH32 targets.

## What Changes

- Add a third "Programmer" interface using GPIO0-9 with dedicated pins: UART TX (GPIO8), UART RX (GPIO9), I2C SDA (GPIO0), I2C SCL (GPIO1), power enable (GPIO7), nRST (GPIO6), SWCLK (GPIO4), SWDIO (GPIO5)
- Add new commands: `PROG_UART_TX`, `PROG_UART_RX`, `PROG_I2C_SDA`, `PROG_I2C_SCL`, `PROG_NRESET`, `PROG_SWDCLK`, `PROG_SWDIO`, `PROG_POWER`
- Support STM32 UART bootloader protocol (AN3155) for flashing
- Support RP2350 UART bootloader protocol for flashing
- Add mode-switching: Programmer interface exclusive during programming (releases GPIO0-9 from other functions)
- Include self-test loopback on GPIO8↔GPIO9 for verification

## Capabilities

### New Capabilities
- `programmer-uart`: UART-based flash programming for STM32, RP2350, NRF, CH32 targets using GPIO8 (TX) and GPIO9 (RX)
- `programmer-swd`: SWD debug interface using PIO1 on GPIO4 (SWCLK) and GPIO5 (SWDIO) — Phase 2
- `programmer-control`: Power enable (GPIO7), nRST control (GPIO6), and mode switching for exclusive programmer access

### Modified Capabilities
_(none — this is purely additive)_

## Impact

- **Firmware**: New `agentla_programmer.c/h` modules, updated `agentla_board_settings.h` with PROG_* pin defines, updated `main.c` command parser
- **Hardware**: GPIO0-9 reallocated from adapter capture channels to programmer interface; adapter mode will need reconfiguration (GPIO10-19 remain adapter)
- **Agent**: New flash command set in agent code; existing adapter commands unaffected
- **Compatibility**: Breaking change for anyone using GPIO0-9 via adapter mode — those pins are now exclusively programmer pins
