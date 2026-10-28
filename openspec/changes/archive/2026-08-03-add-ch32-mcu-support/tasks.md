## 1. WCH 1-Wire SDI Protocol (CH32V003)

- [x] 1.1 Create `firmware/pio/agentla_wch_sdi.pio` with reset pulse, bit cell pulse encoding, and edge sampling.
- [x] 1.2 Implement low-level 1-wire SDI byte send/receive functions in `firmware/src/agentla_ch32.c`.

## 2. RISC-V DTM & DMI Protocol Engine

- [x] 2.1 Create `firmware/src/agentla_riscv_dmi.h` defining RISC-V DMI register addresses and flags (`DMCONTROL`, `DMSTATUS`, `ABSTRACTCS`, `COMMAND`, `DATA0`).
- [x] 2.2 Implement `firmware/src/agentla_riscv_dmi.c` providing `riscv_dmi_read()` and `riscv_dmi_write()` over 2-wire SWD and 1-wire SDI.
- [x] 2.3 Implement core halt, resume, register access, and memory bus access routines using DMI abstract commands.

## 3. WCH Flash Controllers & Probe System

- [x] 3.1 Implement target probing and electronic signature (ESIG) chip ID matching for CH32V003 (`0x1FFFF7C4`) and CH32V203 (`0x1FFFF704`).
- [x] 3.2 Implement fast mode flash unlock (`FLASH_MODEKEYR`) and fast page buffer load (`FTPG` + `BUF_LOAD`) programming driver.
- [x] 3.3 Add serial CLI handlers for `CH32 DETECT`, `CH32 FLASH`, and `CH32 ERASE` in `firmware/src/main.c`.

## 4. Verification & Testing

- [x] 4.1 Build firmware and flash to RP2350 target.
- [x] 4.2 Create integration test scripts `test/test_ch32v003.py` and `test/test_ch32v203.py`.
- [x] 4.3 Verify target detection, flash page erase, and firmware programming on CH32V003 target board.
