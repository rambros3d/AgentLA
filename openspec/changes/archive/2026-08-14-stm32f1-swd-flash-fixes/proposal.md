## Why

AgentLA's PIO SWD transport is proven against real hardware: `SWD TEST` successfully reads the STM32F103 Blue Pill DPIDR (0x1BA01477), so the RP2350 probe *does* talk to STM32 over SWD. However, flash programming fails. Root causes were identified by comparing `firmware/src/programmer/target.c` against the Black Magic Probe reference implementation in `references/blackmagic/src/target/stm32f1.c` (drives real F1 hardware daily):

1. `stm32f1_flash_erase()` triggers a full-chip **MASS erase** (`MER=0x04`, then `MER|STRT=0x44`) on every 1 KiB iteration instead of a **page erase** (`PER=0x02`, then `PER|STRT=0x42`). `FLASH ERASE 0x08000000 0x4000` therefore runs 16 complete chip erases.
2. `stm32f1_flash_write()` programs flash with **32-bit AP accesses** (CSW `0x23000012` → Size=WORD) although the F1 flash controller only accepts **halfword** programming (PM0075). This is PGERR-prone and non-conformant.
3. The write loop **never checks PGERR/WRPERR** and never clears EOP between operations. On the F1, error flags latch: every subsequent write fails silently, BSY stays 0, the loop reports success, and `FLASH VERIFY` reads back 0xFF. This is the exact "IDCODE works, flash fails" signature observed on the Blue Pill.
4. The SWD connect sequence tries the ADIv5.2 dormant wake **before** the JTAG-to-SWD switch. The F103's SWJ-DP powers up in **JTAG mode** and has **no dormant support**; the dormant attempt is a no-op and only the fallback rescues the connection. BMP sends the 44-bit JTAG-to-dormant prefix (or falls back to JTAG-to-SWD) precisely for this case.

## Current Findings — Hardware Waveform Debug (Aug 11, 2026)

While verifying the SWD engine against the Blue Pill, the firmware's own logic-analyzer mode (`SWD CAPTURE`, GPIO20/21 loopbacks from SWCLK/SWDIO) uncovered three additional defects that gate all flash work. **These must be fixed and verified before the flash fixes below can be validated on hardware.**

1. **Analyzer pin-base bug (FIXED)** — `board_settings.h` had `INPUT_PIN_BASE 2` (leftover from the old GPIO2-9 design) while `capture.c` maps analyzer channels to GPIO20-29. The PIO sampled 32 pins starting at GPIO2, and `GetBuffer`'s remap computed `pin - INPUT_PIN_BASE = 18` for GPIO20 and applied `(1 << 18)` to an 8-bit byte → **every channel above GPIO9 read as 0, and the remap zeroed the captured data**. All analyzer captures were silently corrupt; this is why the first SWD CAPTURE attempts showed all-zero waveforms. Fixed: `INPUT_PIN_BASE 2 → 20` (matches `pinMap` and `docs/pin-allocation.md`).
2. **SWD CAPTURE timing race (FIXED)** — `swd_read_idcode()` sleeps 60 ms on nRESET before any SWD signaling (10 ms assert + 50 ms deassert wait), while the capture window was 0.4-0.8 ms (4096-8192 samples @ 10 MHz). Debug-log timestamps proved the SWD sequence runs 5-16 ms after the command — the capture finished while pins were still idle. Fixed: `SWD CAPTURE` now calls the protocol primitives directly (`swd_dormant_to_swd()` + `swd_read_dp()`, with JTAG fallback) and uses 65536 samples = 6.5 ms.
3. **Diagnostic marker added (VERIFIED WORKING)** — `SWD CAPTURE` now toggles SWCLK (GPIO4) as a plain GPIO square wave for 0.4 ms at capture start to prove the loopback + capture path. After swapping the GPIO20/21 jumpers, the marker appears on CH1 → **GPIO4 is driven and the capture path works**; SWDIO loopback reads HIGH (pull-up visible) → jumper is good.
4. **CRITICAL OPEN ISSUE — PIO SWD engine stuck generating a bare clock**: the latest 6.5 ms capture shows a perfectly uniform ~39 kHz square wave on SWCLK (256 identical bit cells: 124 samples high / 132 low, zero jitter) filling the **entire** window, with **SWDIO never driven low**. This is not the 20-cycle marker and not a valid SWD sequence. Conclusion: PIO1 SM0 is stuck toggling SWCLK while never driving SWDIO, and `SWD CAPTURE` IDCODE read fails with **error 5 (SWD_ERR_PROTOCOL)**. Prime suspect: a FIFO/autopull synchronization error in `swd_write_bits()`/`swd_transfer()` (the `bit_count==32` filler word and trailing dummy) causing the SM to `out pc` into a garbage origin and spin in a `seq_out`-style loop with a corrupt bit count. Root-cause this before anything else.
5. **Connect-sequence ordering is now MORE critical**: the direct-primitives path (dormant-first, JTAG fallback) that `SWD CAPTURE` uses fails on the F103 with error 5, reinforcing that the JTAG-to-SWD switch must be attempted before the dormant wake for SWJ-DP (JTAG-default) targets.

## What Changes

- **STM32F1 page erase**: Rewrite `stm32f1_flash_erase()` to erase 1 KiB pages via `PER=0x02` + `AR=addr` + `PER|STRT=0x42`, matching BMP. Add a distinct explicit mass-erase path (`MER|STRT=0x44`) for full-chip erase commands.
- **STM32F1 halfword programming**: Rewrite `stm32f1_flash_write()` to program with 16-bit CSW size (`ADIV5_AP_CSW_SIZE_HALFWORD`) so each flash write is a true halfword access.
- **Flash error handling**: Clear `EOP` in FLASH_SR before every erase/write operation (BMP pattern) and check `PGERR`/`WRPRTERR` after completion; abort with a descriptive error instead of silently succeeding.
- **SWD connect sequence**: For Cortex-M targets with SWJ-DP (JTAG-default), attempt the JTAG-to-SWD switch before (or in addition to) the dormant wake, per BMP `adiv5_swd.c`. Keep dormant-first only where it is correct (ADIv5.2+ dormant-capable targets).
- **Target detection**: Add STM32F1 part IDs `0x410` (medium density — Blue Pill C8T6), `0x412` (low density), `0x414` (high density), `0x430` (XL density) so the Blue Pill identifies as `STM32F103`.
- **SWD clock accounting**: Correct the PIO clock divider for 8 PIO cycles per SWD bit (documented actual bit rate = SM clock / 8), or document the discrepancy.

## Capabilities

### New Capabilities

_(None — all fixes modify existing spec-level behavior.)_

### Modified Capabilities

- `flash-algorithms`: STM32F1 flash algorithm requirements change — page erase instead of mass erase, halfword (16-bit) flash programming, and EOP/PGERR/WRPRTERR handling that detects and reports failures.
- `dormant-to-swd`: SWD connect sequence requirements change — JTAG-to-SWD switch must precede or accompany the dormant wake for SWJ-DP (JTAG-default, non-dormant) targets like STM32F1.
- `target-detection`: STM32F1 part-ID table expanded with medium/low/high/XL density IDs so the Blue Pill C8T6 (0x410) is identified as STM32F103.

## Impact

- **Code affected**: `firmware/src/programmer/target.c` (`stm32f1_flash_erase`, `stm32f1_flash_write`, `stm32f1_unlock_flash`, `stm32f1_wait_flash_bsy`, part-ID table), `firmware/src/programmer/swd.c` (`swd_read_idcode` connect order, `swd_write_bits`/`swd_transfer` FIFO layout), possibly `firmware/src/programmer/swd.h` (CSW size constants), `firmware/src/programmer/tap_pio.c` (DMA/FIFO diagnostics).
- **Already changed (waveform debugging)**: `firmware/src/board_settings.h` (`INPUT_PIN_BASE 2→20`), `firmware/src/main.c` (`SWD CAPTURE` direct-primitive path, 65536-sample window, diagnostic marker).
- **Tests**: `test/test_blink_bluepill.py` (erase/write/verify flow, expected response strings), new error-path assertions reading back FLASH_SR after writes.
- **Dependencies**: None. Fixes are validated against `references/blackmagic/src/target/stm32f1.c` (in-repo reference).
- **Risk**: Medium — flash register semantics are hardware-verified against BMP constants; hardware test on a real Blue Pill required.
