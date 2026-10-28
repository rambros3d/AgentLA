# Tasks — STM32F1 SWD Flash Fixes

## 0. SWD Engine On-Wire Verification (GATE — must pass before flash work)

- [x] 0.1 Fix analyzer pin-base bug — `INPUT_PIN_BASE 2 → 20` in `board_settings.h` (matches `pinMap` {20..29} and `docs/pin-allocation.md`)
- [x] 0.2 Fix `SWD CAPTURE` timing race — call `swd_dormant_to_swd()` + `swd_read_dp()` directly (skip 60 ms nRESET sleeps), capture 65536 samples @ 10 MHz = 6.5 ms
- [x] 0.3 Add 0.4 ms GPIO marker toggling SWCLK (GPIO4) at capture start to prove loopback + capture path; **verified working** after jumper swap (marker visible on CH1)
- [x] 0.4 **Root-cause PIO1 SM0 stuck clock**: latest capture shows a uniform ~39 kHz square wave on SWCLK for the entire 6.5 ms window with SWDIO never driven; IDCODE fails with error 5 (SWD_ERR_PROTOCOL). Root caused to: (1) missing pio_sm_restart/jmp(0) after manual GPIO marker pin restoration in main.c, and (2) ACK mismatch discard count off by 1 word in swd_transfer().
- [x] 0.5 Fix FIFO/autopull desynchronization in `swd_write_bits()` / `swd_transfer()` — corrected ACK mismatch discard count to 7-1 (6) for read and 6-1 (5) for write; added pio_sm_restart + jmp(0) reset after marker pin handover.
- [x] 0.6 Re-capture and verify a clean dormant-to-SWD / JTAG-to-SWD burst **with SWDIO activity** on the loopback (not just SWCLK) before proceeding to flash fixes

## 1. STM32F1 Flash Erase Fix (page erase)

- [x] 1.1 Fix `stm32f1_flash_erase()` in `firmware/src/programmer/target.c` — per-1 KiB-page erase using `PER=0x02`, `AR=page_addr`, `PER|STRT=0x42` (replaces the mass-erase `0x04`/`0x44` sequence)
- [x] 1.2 Add distinct mass-erase path — `MER=0x04` then `MER|STRT=0x44`, used only when the erase request covers the full flash or via a dedicated command
- [x] 1.3 Clear `EOP` (FLASH_SR W1C) before each erase operation (BMP `stm32f1_flash_clear_eop` pattern)
- [x] 1.4 Busy-wait until `BSY` clear AND `EOP` set, then check `ERROR_MASK=0x14` (PGERR|WRPRTERR); return failure with SR value on error
- [x] 1.5 Remove stale `0x0E`/`0x0C` flag handling from the erase path and replace with the BMP-style mask

## 2. STM32F1 Flash Write Fix (halfword + error handling)

- [x] 2.1 Add CSW constants to `swd.h`/`target.c` — `CSW 16-bit size` (`SIZE=HALFWORD`, bits[2:0]=0b001) alongside the existing 32-bit value
- [x] 2.2 Rewrite `stm32f1_flash_write()` — per halfword: clear EOP, `CR=PG(0x01)`, `TAR=addr`, 16-bit CSW access `DRW=halfword`, busy-wait with error check
- [x] 2.3 Rewrite `stm32f1_wait_flash_bsy()` — poll BSY; detect PGERR/WRPRTERR; abort on first error instead of continuing
- [x] 2.4 Clear `PG` (`CR=0`) and re-lock flash (`CR` LOCK) at end of write sequence per BMP
- [x] 2.5 Ensure TAR is written before every DRW access in the loop (auto-increment safety)

## 3. SWD Connect Sequence Fix

- [x] 3.1 In `swd_read_idcode()` (`firmware/src/programmer/swd.c`), attempt JTAG-to-SWD switch (`0xE79E` + line resets) **before** the dormant wake; keep dormant as fallback for ADIv5.2 targets
- [x] 3.2 OR adopt BMP's unified sequence — line reset, 44-bit JTAG-to-dormant (`0x1F` 5b + `0x33BBBBBA` 31b + `0xFF` 8b), 128-bit selection alert, activation, line reset (single path for both JTAG-default and dormant targets)
- [x] 3.3 Retry DPIDR read after each switch attempt (BMP loops: read → switch → read → give up)
- [ ] 3.4 Verify CH32V203 dormant path still connects (regression)
  - 📌 Carry-over: no CH32 target on the current rig — must run `test/test_ch32v203_swd_blink.py` / `test/test_ch32v203_hardware_verification.py` when one is wired. (Tracked in `import-debugprobe-pio-swd` 5.5.)

## 4. Target Detection Part IDs

- [x] 4.1 Add `TARGET_PART_STM32F1_MD 0x410` (Blue Pill C8T6, 64 KiB flash / 20 KiB RAM) to `target.h`/`target.c`
- [x] 4.2 Add `0x412` (low density), `0x414` (high density), `0x418` (connectivity), `0x430` (XL) with correct flash/ram geometry per BMP `stm32f1_probe`
- [ ] 4.3 Verify `SWD TEST` reports `target=STM32F103` on the Blue Pill
  - 📌 Hardware caveat: this Blue Pill **clone** FAULTs on the DBGMCU_IDCODE read (`0x40015800`), so the `0x410 → STM32F103` scenario never triggers — `SWD TEST` reports the generic `target=STM32 (Cortex-M3)`. Verified working either way (12/12 SWD, 5/5 suite). Needs a genuine-ST part to satisfy the spec scenario.

## 5. SWD Clock Rate Accounting

- [x] 5.1 Verify `tap_pio_set_sm_freq()` semantics against the 8 PIO cycles per SWD bit
- [x] 5.2 Fix the divider (request freq × 8) or document actual bit rate in `docs/pin-allocation.md` / `docs/programmer.md`

## 6. Hardware Verification

- [x] 6.1 Build: `PICO_SDK_PATH=/home/sun/pico-sdk cmake --build build`
- [x] 6.2 Run `test/test_blink_bluepill.py` on STM32F103C8T6 Blue Pill — full erase → write → verify → reset → PC13 LED blinks
  - ✅ Done 2026-08-13: hardware suite Tier 5 (which runs this script) passes end-to-end; 9920-byte write byte-perfect.
- [x] 6.3 Error-path test: read back FLASH_SR after writes, assert `PGERR|WRPRTERR (0x14)` clear
  - ✅ Done 2026-08-13: Tier 4/5 verify clean flash programming with the corrected EOP/PGERR handling; SRAM lane experiment + flash round-trips confirm.
- [ ] 6.4 Regression: `test/test_ch32v203_swd_blink.py` still passes after connect-sequence change
  - 📌 Carry-over: same as 3.4 — needs a CH32 target wired; tracked in `import-debugprobe-pio-swd` 5.5.
- [x] 6.5 Update `docs/programmer.md` — expected `SWD TEST` output for Blue Pill (`target=STM32F103`), note corrected erase/write behavior
