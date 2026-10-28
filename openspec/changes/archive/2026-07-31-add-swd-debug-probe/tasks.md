## 1. PIO SWD Engine

- [x] 1.1 Port debugprobe probe.pio to agentla_swd.pio (PIO1, SWCLK sideset, SWDIO bidirectional)
- [x] 1.2 Implement swd_init() — configure PIO1 state machine, claim pins GPIO 6-7
- [x] 1.3 Implement probe_write_bits() — clock out 1-256 bits on SWDIO with SWCLK
- [x] 1.4 Implement probe_read_bits() — sample 1-256 bits from SWDIO on SWCLK rising edge
- [x] 1.5 Implement probe_set_swclk_freq() — PIO clock divider for 1-4 MHz
- [x] 1.6 Implement swd_line_reset() — 50+ high cycles + 2 idle cycles
- [x] 1.7 Implement probe_turnaround() — 1-cycle TRN for SWDIO direction change
- [x] 1.8 Create agentla_swd.h header with public API declarations

## 2. SWD Protocol Layer

- [x] 2.1 Implement swd_read_idcode() — read DPIDR register via SWD
- [x] 2.2 Implement swd_write_dp() — write DP register (SEL, RDBUFF, etc.)
- [x] 2.3 Implement swd_read_dp() — read DP register
- [x] 2.4 Implement swd_write_ap() — write AP register with CSW/TAR/DRW
- [x] 2.5 Implement swd_read_ap() — read AP register
- [x] 2.6 Implement swd_jtag_to_swd() — send 0xE79E switching sequence
- [x] 2.7 Handle SWD ACK responses (OK/WAIT/FAULT) with retry logic

## 3. Target Detection

- [x] 3.1 Implement target_detect() — read IDCODE and extract designer ID
- [x] 3.2 Add STM32 designer ID patterns (0x043x family)
- [x] 3.3 Add RP2040/RP2350 designer ID patterns (0x483x family)
- [x] 3.4 Implement auto-select flash algorithm based on IDCODE

## 4. Flash Algorithms

- [x] 4.1 Create flash_algorithm_t interface (name, idcode, erase, write, read)
- [x] 4.2 Implement STM32F1 flash algorithm (1KB sectors, flash key sequence)
- [x] 4.3 Implement STM32F4 flash algorithm (variable sectors, double-word programming)
- [x] 4.4 Implement STM32G0 flash algorithm (2KB pages, half-page programming)
- [x] 4.5 Implement RP2040/RP2350 flash algorithm (4KB sectors, XIP controller)
- [x] 4.6 Create algorithm registry table with auto-detection mapping

## 5. USB HID Interface

- [x] 5.1 Create USB HID device descriptor (VID 0x1209, PID 0x414D, 64-byte reports)
- [x] 5.2 Implement TinyUSB HID setup with CMSIS-DAP report descriptor
- [x] 5.3 Implement USB HID IN/OUT endpoint handlers
- [x] 5.4 Add USB enumeration and configuration in main.c

## 6. CMSIS-DAP Protocol

- [x] 6.1 Implement DAP_Info command (firmware version, vendor, capabilities)
- [x] 6.2 Implement DAP_Connect/Disconnect commands
- [x] 6.3 Implement DAP_Transfer command (SWD read/write with batching)
- [x] 6.4 Implement DAP_SWJ_Clock command (speed negotiation)
- [x] 6.5 Implement DAP_SWJ_Sequence command (reset, JTAG2SWD)
- [x] 6.6 Implement DAP_ResetTarget command
- [x] 6.7 Implement DAP_TransferConfigure command (idle cycles, wait retry)
- [x] 6.8 Create CMSIS-DAP packet parser/dispatcher

## 7. Flash Programming Commands

- [x] 7.1 Implement flash_erase command (sector/page erase via AP)
- [x] 7.2 Implement flash_write command (programming via AP)
- [x] 7.3 Implement flash_read command (read via AP)
- [x] 7.4 Implement flash_verify command (compare written data)

## 8. CLI Commands

- [x] 8.1 Add MODE SWD ON/OFF/STATUS commands
- [x] 8.2 Add SWD IDCODE command
- [x] 8.3 Add SWD READ/WRITE commands (DP/AP register access)
- [x] 8.4 Add SWD RESET command
- [x] 8.5 Add SWD TEST command (connectivity check)
- [x] 8.6 Update IDENTIFY to report SWD capabilities

## 9. Integration

- [x] 9.1 Update CMakeLists.txt with new source files
- [x] 9.2 Update agentla_board_settings.h with SWD constants
- [x] 9.3 Add SWD mode state management (shared pins with UART)
- [x] 9.4 Add mode conflict detection (SWD vs UART bootloader)
- [x] 9.5 Update pin-allocation.md documentation

## 10. Testing

- [ ] 10.1 Unit test: PIO SWD engine with loopback (GPIO 6-7 jumper)
- [ ] 10.2 Unit test: SWD protocol layer with Blue Pill target
- [ ] 10.3 Integration test: USB HID enumeration as CMSIS-DAP device
- [ ] 10.4 Integration test: OpenOCD connection and target detection
- [ ] 10.5 Integration test: Flash erase/write/read on Blue Pill
- [ ] 10.6 Integration test: pyOCD connection and flash programming
- [ ] 10.7 Integration test: STM32F4 target (if available)
- [ ] 10.8 Integration test: RP2040/RP2350 target (if available)

## 11. Documentation

- [x] 11.1 Update docs/programmer.md with SWD commands
- [x] 11.2 Add OpenOCD configuration examples
- [x] 11.3 Add pyOCD usage examples
- [x] 11.4 Document pin connections for common targets

## 12. Simultaneous Interface Support (NEW)

- [x] 12.1 Analyzer (GPIO20-29) + Adapter (GPIO10-19) + SWD (GPIO4-6) all active simultaneously
- [x] 12.2 SWD lazy auto-init on first command (no MODE SWD ON required)
- [x] 12.3 UART conflict detection in SWD auto-init (GPIO4-5 shared)
- [x] 12.4 MODE command removed — all modes auto-activate when needed
- [x] 12.5 Build metadata: BUILD_TIMESTAMP and GIT_HASH in IDENTIFY output
- [x] 12.6 Dynamic git hash via CMake execute_process()

## 13. Known Issues (TODO)

- [x] 13.1 USB CDC serial not responding after flash — firmware hangs or USB init fails (fixed: disabled stdio_usb, using TinyUSB directly)
- [x] 13.2 BOOTSEL command via serial not working (fixed: fflush + uart_tx_wait_blocking) — reset_usb_boot may need fix for RP2350
- [ ] 13.3 picotool reboot -u not implemented — firmware lacks vendor USB interface for picotool bootsel (CRITICAL: blocks reflashing without physical BOOTSEL press)
- [ ] 13.4 Mass storage flash unreliable — /media/sun/RP2350/ is stale inside container; real mount is /run/media/sun/RP2350/ on host filesystem
- [x] 13.5 USB HID data write times out (fixed: tud_hid_ready check + CMSIS-DAP verified working) (ETIMEDOUT) on host — firmware reaches main loop (USB enumerates) but does not process HID endpoint data; OpenOCD CMD_INFO sometimes works, sometimes doesn't; likely cause is kernel usbhid driver conflicting with raw hidraw access OR TinyUSB endpoint not being processed by firmware
- [x] 13.6 GPIO 0/1 conflict: stdio_init_all() sets UART0 on GPIO 0/1, but adapter_init() reconfigures them to I2C (ADAPTER_I2C_SCL=0, ADAPTER_I2C_SDA=1), breaking UART console output after init (fixed: stdio_uart_init_full on GPIO 12/13)
- [x] 13.7 GPIO 4/5 conflict: PROG_SWDCLK=4, PROG_SWDIO=5 are also ADAPTER_UART_TX/RX pins; adapter_init() sets them to UART function before SWD init reconfigures them to PIO (fixed: adapter_init_i2c_only + adapter_uart_init_lazy + SWD busy check)
