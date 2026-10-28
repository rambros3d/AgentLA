## 1. Hardware Debugging & Protocol Recovery Engine

- [x] 1.1 Add automatic SWD clock frequency fallback (1000 kHz → 500 kHz → 100 kHz) on persistent `SWD_ACK_WAIT` (`011`) responses in `firmware/src/programmer/swd.c`
- [x] 1.2 Implement `cmd_swd_diag` in `firmware/src/main.c` to report real-time pin levels (SWCLK, SWDIO, nRESET) and last captured ACK status over CDC
- [x] 1.3 Add 100MHz logic analyzer signal capture script (`scratch/analyze_swd_capture.py`) for offline SWCLK/SWDIO waveform analysis

## 2. CDC Serial Command Integration

- [x] 2.1 Add `cmd_stm32_test` handler in `firmware/src/main.c` supporting `STM32 TEST` command
- [x] 2.2 Implement DPIDR, `DP_CTRLSTAT`, `AHB-AP`, DBGMCU `0x410`, and Flash size reporting in `cmd_stm32_test`
- [x] 2.3 Register `STM32` command in `process_command` dispatch loop in `main.c`

## 3. Automated 5-Tier Hardware Test Suite

- [x] 3.1 Create `test/test_stm32f1_hardware_suite.py` test harness script
- [x] 3.2 Implement Tier 1 (Transport & DP/AP framing: DPIDR `0x1BA01477`, `DP_CTRLSTAT` power-up, AP IDR `0x04770001`, and frequency sweep 100 kHz - 5 MHz)
- [x] 3.3 Implement Tier 2 (Cortex-M3 CPU control: DHCSR halt/resume, register inspection, single-step execution, and reset-vector catch)
- [x] 3.4 Implement Tier 3 (AHB Memory & Chip ID: SRAM 8/16/32-bit read/write, DBGMCU ID `0x410`, Flash Size `0x1FFFF7E0`, and 96-bit UID `0x1FFFF7E8`)
- [x] 3.5 Implement Tier 4 (STM32F1 FPEC Flash Driver: key unlock, 1 KiB page erase, 16-bit CSW halfword programming, and latched PGERR/WRPRTERR W1C clear)
- [x] 3.6 Implement Tier 5 (End-to-End Application Flashing & Logic Analyzer PC13 LED signal sampling on GPIO18 for 3 seconds)

## 4. Documentation & Verification

- [x] 4.1 Update `docs/programmer.md` with 5-tier test suite instructions, signal debugging commands, wiring diagram, and expected outputs
- [x] 4.2 Run full hardware verification & SWD signal debugging pipeline against connected Blue Pill target
