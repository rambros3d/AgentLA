## 1. Firmware SWD Turnaround Tuning & Reset Catch

- [x] 1.1 Add `swd_set_turnaround_cycles(uint8_t cycles)` to `firmware/src/programmer/swd.c` and `swd.h`
- [x] 1.2 Add CDC serial command `SWD TURNAROUND <1|2>` handler in `firmware/src/main.c`
- [x] 1.3 Add vector-catch connect-under-reset helper in `firmware/src/programmer/swd.c`

## 2. Autonomous Multi-Phase Diagnostic Harness

- [x] 2.1 Create `test/test_stm32f1_autonomous_debug.py` executable script
- [x] 2.2 Implement Phase 1: Prescaler & Turnaround Cycle Sweep (50 kHz - 1000 kHz, 1 vs 2 cycles)
- [x] 2.3 Implement Phase 2: Connect-Under-Reset Vector Catch (`nRESET` pulse + `VC_CORERESET` halt)
- [x] 2.4 Implement Phase 3: FPEC Flash Controller Sector Programming & Readback Verification
- [x] 2.5 Implement Phase 4: Logic Analyzer Signal Sampling on `GPIO18` (PC13 LED 1 Hz toggle check)

## 3. Autonomous Debugging & Verification Run

- [x] 3.1 Build firmware and flash to RP2350 probe using `picotool`
- [x] 3.2 Run `python3 test/test_stm32f1_autonomous_debug.py /dev/ttyACM0` autonomously until Blue Pill is fully verified
