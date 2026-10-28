## 1. Software Bitbanging SWD Implementation

- [x] 1.1 Add `swd_bitbang_read_idcode()` in `firmware/src/programmer/swd.c` and declare in `swd.h`
- [x] 1.2 Add CDC serial command `SWD BITBANG` handler in `firmware/src/main.c`
- [x] 1.3 Build firmware and flash to RP2350 probe using `picotool`

## 2. Empirical Isolation & Diagnostic Execution

- [x] 2.1 Run `SWD BITBANG` over CDC serial `/dev/ttyACM0` and record ACK / IDCODE responses
- [x] 2.2 Run `SWD CAPTURE` to record bitbanging logic analyzer waveforms on GPIO20/21
- [x] 2.3 Compare bitbanging timing, pin direction transitions, and clock sampling phases against PIO

## 3. PIO State Machine Refactoring & Verification

- [x] 3.1 Update `firmware/pio/swd.pio` and `firmware/src/programmer/swd.c` based on bitbanging findings
- [x] 3.2 Build and flash updated firmware to RP2350 probe
- [x] 3.3 Run `SWD IDCODE` and `python3 test/test_stm32f1_autonomous_debug.py /dev/ttyACM0` until 100% verified
