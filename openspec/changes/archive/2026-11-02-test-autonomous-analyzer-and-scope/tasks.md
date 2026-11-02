## 1. Target Signal Generator Firmware

- [x] 1.1 Create `test-firmware/target-pico2-generator` CMake project targeting RP2350.
- [x] 1.2 Implement PWM clock output on GPIO 20 (10 kHz, 50% duty).
- [x] 1.3 Implement PWM test output on GPIO 21 (1 kHz, 30% duty).
- [x] 1.4 Implement analog reference output on GPIO 26.
- [x] 1.5 Implement UART periodic transmitter on GPIO 12 with full-duplex echo on GPIO 13.
- [x] 1.6 Build `target_pico2_generator.uf2` and verify binary output.

## 2. Autonomous Test Runner

- [x] 2.1 Implement `test/test_autonomous_hardware_verification.py` connection management and handshake.
- [x] 2.2 Add Oscilloscope test suite (`ADCSCOPE pin=26`, `27`, `28`) verifying voltage bounds, noise floor, and sample stream.
- [x] 2.3 Add Logic Analyzer test suite (`CAPTURE pin=20`, `21`) calculating observed frequency and duty cycle.
- [x] 2.4 Add Protocol Decoder test suite verifying live UART packet parsing from raw logic capture.
- [x] 2.5 Add Adapter Loopback test suite (`UART_WRITE` and `UART_READ`) validating data integrity.

## 3. Verification & Validation

- [x] 3.1 Flash target Pico 2 with `target_pico2_generator.uf2` via USB.
- [x] 3.2 Execute complete test suite against live hardware and verify 100% pass rate.
- [x] 3.3 Log test results and metrics to `docs/hardware-verification-report.md`.
