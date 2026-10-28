## 1. Board Settings and Pin Definitions

- [x] 1.1 Add PROG_* pin defines to `agentla_board_settings.h` (PROG_UART_TX=8, PROG_UART_RX=9, PROG_I2C_SDA=0, PROG_I2C_SCL=1, PROG_NRESET=6, PROG_SWDCLK=4, PROG_SWDIO=5, PROG_POWER=7)
- [x] 1.2 Add PROGRAMMER_MODE state variable and PROG_MODE_IS_ACTIVE macro
- [x] 1.3 Add programmer-specific constants (PROG_BAUD_RATE=115200, PROG_POWER_DELAY_MS=100, PROG_NRESET_PULSE_MS=100)

## 2. Programmer Module Structure

- [x] 2.1 Create `agentla_programmer.h` with public API declarations (prog_init, prog_enter_mode, prog_exit_mode, prog_power, prog_nreset, prog_uart_test, prog_stm32_sync, prog_stm32_write, prog_stm32_verify, prog_rp2350_sync, prog_rp2350_load, prog_nrf_sync, prog_ch32_sync)
- [x] 2.2 Create `agentla_programmer.c` with static state and internal helpers (prog_uart_write_byte, prog_uart_read_byte, prog_uart_write, prog_uart_read, prog_send_ack, prog_wait_ack)
- [x] 2.3 Add `#include "agentla_programmer.h"` to `main.c`

## 3. Programmer Mode Control

- [x] 3.1 Implement `prog_enter_mode()`: reconfigure GPIO0-9 as programmer pins, init UART1, set PROG_MODE_IS_ACTIVE
- [x] 3.2 Implement `prog_exit_mode()`: release GPIO0-9, deinit UART1, clear PROG_MODE_IS_ACTIVE
- [x] 3.3 Implement `prog_is_active()` accessor function
- [x] 3.4 Add `MODE PROG ON`, `MODE PROG OFF`, `MODE PROG STATUS` command handlers in `main.c`

## 4. Power and Reset Control

- [x] 4.1 Implement `prog_power_on()`: set GPIO7 high, delay 100ms
- [x] 4.2 Implement `prog_power_off()`: set GPIO7 low
- [x] 4.3 Implement `prog_power_status()`: read GPIO7 state
- [x] 4.4 Implement `prog_nreset_low()`: set GPIO6 low (open-drain)
- [x] 4.5 Implement `prog_nreset_high()`: release GPIO6 (input/floating)
- [x] 4.6 Implement `prog_nreset_pulse()`: assert low 100ms then release
- [x] 4.7 Add `PROG POWER ON`, `PROG POWER OFF`, `PROG POWER STATUS` command handlers
- [x] 4.8 Add `PROG NRESET LOW`, `PROG NRESET HIGH`, `PROG NRESET PULSE` command handlers

## 5. UART Self-Test

- [x] 5.1 Implement `prog_uart_test()`: send 0x55 on UART1, read back, verify match
- [x] 5.2 Add `PROG UART TEST` command handler in `main.c`

## 6. STM32 UART Bootloader (AN3155)

- [x] 6.1 Implement `prog_stm32_sync()`: send 0x7F, wait for ACK (0x79), timeout handling
- [x] 6.2 Implement `prog_stm32_write(addr, data, len)`: send write command frame, erase memory, write data with checklist — **NOTE: WRITE command (0x31) does not get ACK from Blue Pill bootloader after SYNC; under investigation**
- [x] 6.3 Implement `prog_stm32_verify(addr, len): read memory, compare, return pass/fail
- [x] 6.4 Add `PROG STM32 SYNC`, `PROG STM32 WRITE`, `PROG STM32 VERIFY` command handlers

## 12. Raw Passthrough Mode for AN3155 Protocol Debugging

- [x] 12.1 Implement `prog_passthrough_forward()` in agentla_programmer.c (UART1 RX → USB CDC)
- [x] 12.2 Add `prog_passthrough_forward()` declaration to `agentla_programmer.h`
- [x] 12.3 Add `prog_passthrough_forward()` call in main.c main loop when raw_passthrough is active
- [x] 12.4 Fixed `uart_putc` in main.c raw passthrough TX path (use `uart_default` macro not `prog_uart` static)
- [x] 12.5 Test raw passthrough end-to-end: send 0x7F SYNC via Python serial, verify 0x79 ACK from Blue Pill UART bootloader
- [x] 12.6 Debug AN3155 WRITE (0x31) protocol — add GetVersion (0x01)+GetID (0x02) prerequisites if needed; implement full AN3155 protocol in Python using raw passthrough

## 7. RP2350 UART Bootloader

- [x] 7.1 Implement `prog_rp2350_sync()`: send sync sequence, wait for ACK
- [x] 7.2 Implement `prog_rp2350_load(addr, data, len)`: send binary load command, write data
- [x] 7.3 Add `PROG RP2350 SYNC`, `PROG RP2350 LOAD` command handlers

## 8. NRF52 UART DFU

- [x] 8.1 Implement `prog_nrf_sync()`: initiate DFU session
- [x] 8.2 Add `PROG NRF SYNC` command handler

## 9. CH32 WCH UART Bootloader

- [x] 9.1 Implement `prog_ch32_sync()`: send WCH sync sequence, wait for ACK
- [x] 9.2 Add `PROG CH32 SYNC` command handler

## 10. Pin Query and IDENTIFY Update

- [x] 10.1 Add `PROG PINS` command handler returning pin assignments
- [x] 10.2 Update IDENTIFY response to include programmer interface info (3 interfaces, programmer pins)

## 11. Build and Verify

- [x] 11.1 Add `agentla_programmer.c` to CMakeLists.txt
- [x] 11.2 Build firmware and verify no compile errors
- [x] 11.3 Verify GPIO0-9 pin directions and functions match design
