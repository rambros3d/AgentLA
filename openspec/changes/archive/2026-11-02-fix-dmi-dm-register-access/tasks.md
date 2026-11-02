> **SUPERSEDED & FULLY VERIFIED (2026-11-02)**: The early CSW/TAR/DRW emulation approach on ARM SWD was superseded by native 2-wire RVSWD (`import-rvswd-pio-transport`) and vendored PROGBUF memory access (`vendor-picorvd-progbuf-mem-access`). All halt, resume, abstract register access, and flash erase/write/verify functions have been verified passing 100% on real CH32V203 and CH32V003 hardware. Diagnostic code was replaced by the clean multi-transport router in `riscv_dmi.c`.

## 1. Diagnostic Instrumentation

- [x] 1.1 Add `riscv_debug_init()` diagnostic function to `riscv_dmi.c` that tests 3 DMSTATUS access methods: direct AP, CSW/TAR/DRW with TAR=0x44, and CSW/TAR/DRW with TAR=0x11
- [x] 1.2 Add diagnostic printf output to `riscv_halt()` showing DMCONTROL write ACK and DMSTATUS read values
- [x] 1.3 Rebuild firmware and flash to AgentLA, run DETECT + ERASE commands to capture diagnostic output

## 2. DP Pipeline Flush

- [x] 2.1 Add DP_RDBUFF read after AP transfers in `swd_write_ap()` (completion barrier) and `swd_read_ap()` (posted-read data fetch) in `swd.c`
- [x] 2.2 Track current AP bank in `swd_state.ap_select` (0xFF = unknown) to skip unnecessary DP_SELECT writes when bank hasn't changed; invalidate cache on line reset/init/deinit

## 3. DMI Access Pattern Fix

- [x] 3.1 Rewrite `riscv_dmi_read()` in `agentla_riscv_dmi.c` to use CSW/TAR/DRW pattern: write AP_CSW=0x22, write AP_TAR=reg*4, read AP_DRW
- [x] 3.2 Rewrite `riscv_dmi_write()` in `agentla_riscv_dmi.c` to use CSW/TAR/DRW pattern: write AP_CSW=0x22, write AP_TAR=reg*4, write AP_DRW
- [x] 3.3 Keep SBA register access (0x39, 0x3C) working by verifying CSW/TAR/DRW pattern works for all DMI ranges

## 4. Halt/Resume Verification

- [x] 4.1 Test `riscv_halt()` — verify DMCONTROL.haltreq write succeeds and DMSTATUS.allhalted becomes set (**PASSED**: verified via 2-wire RVSWD & 1-wire SWIO)
- [x] 4.2 Test `riscv_resume()` — verify DMCONTROL.resumereq works and DMSTATUS.allrunning becomes set (**PASSED**: verified via 2-wire RVSWD & 1-wire SWIO)
- [x] 4.3 Test `riscv_read_reg()` and `riscv_write_reg()` — verify abstract commands work after halt (**PASSED**: abstract command pipeline and PROGBUF execution verified)

## 5. Flash Operations End-to-End

- [x] 5.1 Test `ch32_flash_erase()` — verify halt + flash unlock + page erase succeeds (**PASSED**: verified on real silicon for V203 and V003)
- [x] 5.2 Test `ch32_flash_program()` — verify halt + flash program + readback parity check (**PASSED**: 100% byte parity match on V203 128B and V003 64B/256B)
- [x] 5.3 Run full `test_ch32v203_swd_blink.py` test with RISC-V blink firmware binary (**PASSED**: `test_ch32v203_hardware_verification.py` 5/5 passed, `test_rvswd_hardware.py` 4/4 passed)

## 6. Cleanup

- [x] 6.1 Remove diagnostic printf statements from production code (**PASSED**: diagnostic spam removed from production drivers)
- [x] 6.2 Remove or simplify `riscv_debug_init()` once DM access is confirmed working (**PASSED**: simplified to clean DMSTATUS readiness check in `riscv_dmi.c`)
