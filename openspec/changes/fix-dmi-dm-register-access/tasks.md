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

- [ ] 4.1 Test `riscv_halt()` — verify DMCONTROL.haltreq write succeeds and DMSTATUS.allhalted becomes set
- [ ] 4.2 Test `riscv_resume()` — verify DMCONTROL.resumereq works and DMSTATUS.allrunning becomes set
- [ ] 4.3 Test `riscv_read_reg()` and `riscv_write_reg()` — verify abstract commands work after halt

## 5. Flash Operations End-to-End

- [ ] 5.1 Test `ch32_flash_erase()` — verify halt + flash unlock + page erase succeeds
- [ ] 5.2 Test `ch32_flash_program()` — verify halt + flash program + readback parity check
- [ ] 5.3 Run full `test_ch32v203_swd_blink.py` test with RISC-V blink firmware binary

## 6. Cleanup

- [ ] 6.1 Remove diagnostic printf statements from production code
- [ ] 6.2 Remove or simplify `riscv_debug_init()` once DM access is confirmed working
