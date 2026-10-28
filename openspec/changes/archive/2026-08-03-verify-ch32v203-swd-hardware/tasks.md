## 1. Hardware Verification Setup

- [x] 1.1 Create `test/test_ch32v203_hardware_verification.py` with multi-stage verification framework.
- [x] 1.2 Implement serial handshake, timeout management, and automatic process lock cleanup routines.

## 2. RV-SWD Target Probe & Register Verification

- [x] 2.1 Test `IDENTIFY` device serial response.
- [x] 2.2 Test 2-wire RV-SWD probe and ESIG chip signature matching (`0x1FFFF704`).

## 3. Flash Programming & Readback Verification

- [x] 3.1 Test fast page sector erase (`CH32 ERASE 0x08000000 128`).
- [x] 3.2 Program 128-byte CH32V203 test binary payload (`CH32 FLASH 0x08000000 <hex_data>`).
- [x] 3.3 Verify programmed payload via `FLASH READ` and execute byte-by-byte parity check.
