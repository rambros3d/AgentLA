## 1. PIO Program Rewrite

- [x] 1.1 Create new `agentla_swd.pio` with 7 modular sub-programs using `.origin` directives (start, seq_out_turnaround, seq_out, seq_in_turnaround, seq_in, adiv5_check_ack, turnaround_float_to_drive) adapted from MioLink's `pio_swd_miolink_pico.pio`
- [x] 1.2 Update PIO side-set to 1-bit (SWCLK only, no level shifter) and map IN/OUT/Set to GPIO5 (SWDIO)
- [x] 1.3 Add 3-cycle delay after each edge for timing margin (max SWD freq = sys_clk/8)
- [x] 1.4 Enable autopull and autopush with 32-bit threshold
- [x] 1.5 Verify PIO program fits in 32 instruction slots on PIO1 SM0

## 2. PIO/DMA Integration Layer

- [x] 2.1 Create `agentla_tap_pio.h` with PIO sub-program origin addresses, DMA channel definitions, and helper functions
- [x] 2.2 Create `agentla_tap_pio.c` with DMA send/receive functions adapted from MioLink's `tap_pio.c`
- [x] 2.3 Implement `tap_pio_dma_send_uint32()` — DMA transfer from memory to PIO TX FIFO
- [x] 2.4 Implement `tap_pio_dma_send_recv_uint32()` — simultaneous DMA send and receive
- [x] 2.5 Implement `tap_pio_set_sm_freq()` — configure PIO clock divider for SWD frequency
- [x] 2.6 Implement `tap_pio_init()` — claim DMA channels, configure PIO SM, set pin directions

## 3. SWD Protocol Rewrite

- [x] 3.1 Rewrite `swd_write_bits()` to use PIO sub-program origins instead of command dispatch
- [x] 3.2 Rewrite `swd_read_bits()` to use PIO sub-program origins instead of command dispatch
- [x] 3.3 Update `swd_line_reset()` to use 60 HIGH cycles + 4 idle (from BMP/MioLink)
- [x] 3.4 Fix turnaround timing by delegating to PIO turnaround sub-programs
- [x] 3.5 Update `swd_transfer()` to read ACK from PIO RX FIFO after hardware check

## 4. Dormant-to-SWD Sequence

- [x] 4.1 Implement dormant wake sequence in C code (not PIO) based on BMP's `adiv5_swd.c`
- [x] 4.2 Add 128-bit Selection Alert sequence
- [x] 4.3 Add activation code after Selection Alert
- [x] 4.4 Update `swd_init()` to call dormant wake before first DPIDR read

## 5. Board Settings & Build

- [x] 5.1 Update `agentla_board_settings.h` with PIO1 pin definitions and DMA channel assignments
- [x] 5.2 Update CMakeLists.txt to include new `agentla_tap_pio.c` source file
- [x] 5.3 Verify PIO program compiles with pioasm
- [x] 5.4 Update `agentla_swd.h` with new function signatures and constants

## 6. Testing & Validation

- [x] 6.1 Build firmware and verify no compilation errors
- [x] 6.2 Flash firmware to RP2350 and verify USB enumeration (CDC + HID)
- [x] 6.3 Test SWD IDCODE with Blue Pill (STM32F103) via CDC CLI
- [x] 6.4 Test SWD READ/WRITE with Blue Pill via CDC CLI
- [x] 6.5 Test CMSIS-DAP HID interface with OpenOCD
- [x] 6.6 Verify logic analyzer and adapter interfaces still work
- [x] 6.7 Run existing test suite to verify no regressions
