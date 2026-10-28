## Context

AgentLA programs STM32F1 targets through the PIO SWD engine (`firmware/src/programmer/swd.c`, MioLink architecture) and a register-level flash driver in `firmware/src/programmer/target.c`. Hardware testing on an STM32F103C8T6 Blue Pill showed:

- `SWD TEST` → **PASS**, reads DPIDR `0x1BA01477` → the SWD transport, JTAG-to-SWD fallback, and ACK/parity handling all work against a real Cortex-M3.
- Flash erase/write/verify → **fail**.

Because the CH32V203 path was never hardware-verified, the flash layer was the first code ever exercised against real ARM silicon — and it has never worked there.

The in-repo Black Magic Probe source (`references/blackmagic/src/target/stm32f1.c`, `adiv5.h`) is the authoritative reference: it is the same ADIv5 register protocol, validated on thousands of real F1 devices.

## Waveform Debug Findings (Aug 11, 2026)

The RP2350 probe's own logic-analyzer mode (`SWD CAPTURE` on GPIO20/21, loopback from SWCLK/SWDIO) was used to observe the SWD engine on the wire. Three defects were found and two are fixed; one critical issue remains.

### Fixed: Analyzer pin-base bug

`board_settings.h` defined `INPUT_PIN_BASE 2` (leftover from the old GPIO2-9 design) but `capture.c`'s `pinMap` is `{20..29}`. Consequences:
- PIO sampled a 32-bit word starting at GPIO2; 8-channel DMA kept only the low byte (GPIO2-9).
- `GetBuffer()` remapped with `lastPin = pin - INPUT_PIN_BASE` = `20 - 2 = 18` and applied `(oldValue & (1 << 18))` to an 8-bit byte → **always 0, and it overwrote the raw byte with 0**, destroying captured data.
- Every analyzer channel ≥ GPIO10 read as 0 — this silently corrupted every capture, which is why initial SWD CAPTURE runs showed all-zero waveforms even during active SWD transactions.

**Fix applied**: `INPUT_PIN_BASE 2 → 20` in `firmware/src/board_settings.h` (matches `pinMap` and `docs/pin-allocation.md`).

### Fixed: SWD CAPTURE timing race

`swd_read_idcode()` sleeps **60 ms on nRESET** (10 ms assert + 50 ms deassert) before emitting any SWD signaling. The original capture window was 0.4 ms (4096 samples @ 10 MHz); timestamped debug logs proved the SWD sequence runs ~5-16 ms after the command — the capture window ended before any signal appeared on the wire.

**Fix applied**: `SWD CAPTURE` now calls the primitives directly (`swd_dormant_to_swd()` + `swd_read_dp()` with JTAG fallback) and captures 65536 samples = 6.5 ms window.

### Verified: loopback + capture path

A 0.4 ms GPIO marker toggling SWCLK (GPIO4) was added at capture start. After swapping the GPIO20/21 jumpers the marker appeared on CH1 — proving GPIO4 is driven, the loopback jumpers connect, and the analyzer path works. SWDIO loopback reads HIGH (pull-up visible through the GPIO5→GPIO21 jumper).

### OPEN ISSUE: PIO1 SM0 stuck generating a bare clock

The latest 6.5 ms capture shows a **perfectly uniform ~39 kHz square wave** on SWCLK — 256 identical bit cells (124 samples high / 132 low, zero jitter) — filling the entire window, with **SWDIO never driven low** (pull-up only). This is neither the 20-cycle marker (ends at 0.4 ms) nor a real SWD protocol burst (which is bursty and drives SWDIO during the alert/request phases). `SWD CAPTURE` IDCODE fails with **error 5 = SWD_ERR_PROTOCOL**.

Interpretation: after the marker restores GPIO4 to PIO1 (`pio_gpio_init` + `pio_sm_set_consecutive_pindirs`), the state machine appears stuck in a `seq_out`-style loop — toggling SWCLK via the side-set while never executing the `set pindirs, 1` that would drive SWDIO, and never terminating. Prime suspect: **FIFO/autopull desynchronization** in `swd_write_bits()` / `swd_transfer()` (the `bit_count==32` filler word and trailing dummy were added to work around autopull behavior; a mismatch makes the SM `out pc, 32` jump into a garbage origin and spin with a corrupt bit count).

Next diagnostic step (before any flash work):
1. Dump PIO1 SM0 state (PC, TX/RX FIFO levels, TXSTALL, EXEC_STALL, DMA timeout flag) via the `DIAG` command right after `SWD CAPTURE` — this pinpoints the exact instruction the SM is stuck at.
2. If stuck in a `seq_out` loop: verify the exact FIFO word consumption against the PIO program, fix `swd_write_bits()`/`swd_transfer()` buffer layouts, and re-capture.
3. Verify a clean dormant-to-SWD / JTAG-to-SWD burst with SWDIO activity appears on the loopback before proceeding to flash fixes.

## Goals / Non-Goals

**Goals:**
- Make `FLASH ERASE` / `FLASH WRITE` / `FLASH VERIFY` work correctly on STM32F1 Blue Pill hardware.
- Match BMP's proven F1 flash sequence (page erase, halfword programming, EOP/PGERR handling).
- Fix the SWD connect order so SWJ-DP (JTAG-default) targets like STM32F1 connect deterministically.
- Report flash errors loudly instead of silently succeeding.
- Identify the Blue Pill C8T6 (DBGMCU ID `0x410`) correctly.

**Non-Goals:**
- Implementing a RAM-resident flash loader (the ST-Link/OpenOCD speed approach). Correctness first; a register-level driver is acceptable initially.
- Adding RDP / option-byte programming (separate capability).
- Changing the CMSIS-DAP interface.
- Adding JTAG mode (MioLink reference exists, separate effort).

## Decisions

### 1. STM32F1 Erase — page erase, not mass erase

**Decision**: Rewrite `stm32f1_flash_erase()` to erase 1 KiB pages one at a time: `CR=PER(0x02)`, `AR=page_addr`, `CR=PER|STRT(0x42)`, then busy-wait. Add a separate explicit mass-erase path (`MER=0x04` then `MER|STRT=0x44`) used only for whole-chip erase requests.

**Rationale**: Current code sends `CR=0x04` then `CR=0x44` — this is the **mass erase** sequence, executed once per 1 KiB step, so `FLASH ERASE <range>` mass-erases the entire chip N times. BMP uses PER for page erase (stm32f1.c L1154-1159) and reserves MER for `mass_erase` (L1228-1230). F1 `FLASH_CR` bits per BMP: `PER=0x02`, `MER=0x04`, `STRT=0x40`.

**Implementation** (mirrors BMP):
```
for each 1 KiB page in [addr, addr+size):
    clear EOP in FLASH_SR
    CR = PER (0x02)
    AR = page_addr
    CR = PER|STRT (0x42)
    busy-wait for BSY clear, check error mask
```

### 2. STM32F1 Write — halfword programming with 16-bit CSW

**Decision**: Program flash halfword-by-halfword using a **16-bit** AP access: CSW with `SIZE=HALFWORD` (bits[2:0]=0b001 → `0x23000011`), `TAR=addr`, `DRW=halfword`.

**Rationale**: BMP programs genuine ST F1 parts with `ALIGN_16BIT` (`psize = ALIGN_16BIT` unless `STM32F1_TOPT_32BIT_WRITES` is set — that flag is only for GD32/AT32 clones). The F1 flash controller (PM0075) only accepts halfword programming; the current `CSW=0x23000012` encodes `SIZE=WORD` (32-bit), which is non-conformant and PGERR-prone. Reads and register accesses may stay 32-bit; only flash *programming* needs 16-bit width.

**Note on lane mapping (CORRECTED 2026-08-13 — supersedes the earlier claim below)**: The
F1 AHB-AP does **NOT** right-align sub-word data. A 16-bit access must be **pre-positioned
in the DRW byte lane selected by `addr bit[1]`**: bits [15:0] for `addr%4==0`, bits [31:16]
for `addr%4==2`. This mirrors BMP's `adiv5_pack_data()` (`*data = value << (8U * (dest & 2U))`).
The earlier assumption that `halfword = data[i] | (data[i+1] << 8)` already packs into the
correct lane was **proven wrong on hardware** — halfwords at `addr%4==2` programmed as
`0x0000`, causing the Tier-5 verify mismatch (`expected 0x20 got 0x00`). Fix shipped in
`target.c`: `swd_write_ap(AP_DRW, (uint32_t)halfword << (8U * ((addr + i) & 2U)))`. See
`import-debugprobe-pio-swd` design.md → "Root cause: 16-bit byte-lane positioning".

### 3. Flash error handling — clear EOP before, check errors after

**Decision**: Adopt the BMP pattern (`stm32f1_flash_clear_eop` L1092-1096, `stm32f1_flash_busy_wait` L1098-1122):
- Before every program/erase op: read FLASH_SR, write `EOP (0x20)` back (W1C) to clear stale flags.
- After every op: wait for `BSY` clear **and** `EOP` set, then check `ERROR_MASK = 0x14` (PGERR `0x04` | WRPRTERR `0x10`); return failure with the SR value if set.
- The write loop SHALL abort on the first error rather than continuing to "succeed".

**Rationale**: On the F1, once PGERR/WRPRTERR is set, programming is inhibited until the flags are cleared. The current `stm32f1_wait_flash_bsy()` only polls BSY, so a single PGERR makes every subsequent write fail silently while the CLI reports `OK FLASH WROTE`. This is the root cause of the observed "flash fails" with misleading success responses.

### 4. SWD connect order — JTAG-to-SWD before dormant for SWJ-DP targets

**Decision**: In `swd_read_idcode()`, attempt the **JTAG-to-SWD switch before the dormant wake** for targets whose DP is a JTAG-default SWJ-DP (STM32F1, most Cortex-M). Order:
1. Line reset (60 high + 4 idle)
2. JTAG-to-SWD select `0xE79E` (16 bits)
3. Line reset
4. Read DPIDR; if no response, retry with the dormant wake sequence (for ADIv5.2 dormant-capable targets such as CH32V203)

**Alternative**: BMP sends the 44-bit **JTAG-to-dormant** prefix (`0x1F` 5b + `0x33BBBBBA` 31b + `0xFF` 8b) before the 128-bit selection alert, which works for both JTAG-default and dormant targets in one shot. Adopt BMP's exact sequence if we want a single unified path; otherwise order the existing primitives JTAG-first with dormant fallback.

**Rationale**: The F103 SWJ-DP powers up in JTAG mode (ADIv5 B5.3.2) and has no dormant support. Sending the 128-bit selection alert without first transitioning JTAG→dormant is a no-op on the F1 (the current code only works because the fallback catches it). The current `swd_dormant_to_swd()` lacks BMP's JTAG-to-dormant prefix.

### 5. Target detection — STM32F1 density part IDs

**Decision**: Add to the part-ID table: `0x410` (medium density — Blue Pill C8T6, 64 KiB), `0x412` (low density), `0x414` (high density — already present but maps to F103), `0x418` (connectivity line), `0x430` (XL density). Populate correct flash_size/ram_size per BMP stm32f1_probe.

**Rationale**: The Blue Pill C8T6 reports DBGMCU ID `0x410`, which is absent from the table, so detection falls through to the generic "STM32 (Cortex-M3)" default. Correct IDs give correct flash/ram geometry and clearer diagnostics.

### 6. SWD clock rate accounting (documentation/perf)

**Decision**: Verify `tap_pio_set_sm_freq()` usage against the 8 PIO cycles per SWD bit (2 instructions × 4 cycles: `out pins,1 side 0 [3]` / `jmp x-- loop side 1 [3]`). If the SM clock is set directly to the requested rate, the actual bit rate is SM clock / 8; request `freq × 8` or document the discrepancy. Purely a performance issue — not a correctness blocker.

## Risks / Trade-offs

- **[F1 flash semantics on clones]** → Some Blue Pill clones use CSM32/CKS32/GD32F103-compatible dies with minor quirks. BMP detects clones via AP IDR and still programs them. Mitigation: follow BMP constants; test on both genuine ST and clone Blue Pills.
- **[16-bit CSW interacts with auto-increment]** → CSW `ADDRINC` stays configured; each op writes TAR explicitly before DRW so residual auto-increment is harmless. Mitigation: verify TAR writes precede every DRW access in the new loops.
- **[Dormant-first ordering change could regress CH32V203]** → The V203 (ADIv5.2-compatible RVSWD) relies on the dormant wake. Mitigation: keep dormant as fallback (or BMP's unified JTAG→dormant→alert sequence) and re-run `test_ch32v203_swd_blink.py` after the change.
- **[Speed]** → Register-level halfword programming over SWD is slow (~minutes for 64 KiB). Accepted for correctness milestone; a RAM-resident flash loader is a follow-up.

## Verification Plan

0. **Gate (must pass first)**: `SWD CAPTURE` shows a clean SWD burst — dormant-to-SWD / JTAG-to-SWD line resets, alert, and a DPIDR transaction **with SWDIO activity** — on the GPIO20/21 loopback. Resolve the stuck-clock issue (task 0.4-0.6) until this passes. `DIAG` must show the SM parked at a valid origin (not spinning in `seq_out`).
1. Build: `PICO_SDK_PATH=/home/sun/pico-sdk cmake --build build` (per note.txt).
2. Hardware, Blue Pill C8T6: `test/test_blink_bluepill.py` — full erase → write → verify → reset → LED blinks.
3. Error-path test: after a write, read back `FLASH_SR` and assert no PGERR/WRPRTERR bits (`0x14`) for a sample of addresses.
4. Regression: `test_ch32v203_swd_blink.py` / `test_ch32v203_hardware_verification.py` for the CH32V203 SWD path.
