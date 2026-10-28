## Technical Approach

1. **PIO State Machine Clock Divider (100 kHz)**:
   - Configure PIO state machine clock divider in `swd_sm_init()` to run at 100 kHz (`clk_sys / (100000 * 8)`).
   - Ensures the STM32F103 SWJ-DP controller properly samples line reset and JTAG-to-SWD `0xE79E` switch code.

2. **Turnaround & ACK Alignment**:
   - Align PIO `swd_seq_in_turnaround` so that SWDIO pin direction change occurs while SWCLK is LOW, matching the bitbanging driver sequence.
   - Adjust `adiv5_check_ack` to match 3-bit ACK (`001` = `0x20000000` in MSB-aligned 32-bit register).

3. **Empirical Logic Analyzer Verification**:
   - Capture GPIO20 (SWCLK) and GPIO21 (SWDIO) using internal LA engine.
   - Confirm SWD request, turnaround phase, and ACK response waveforms match bitbanging timings.

## Risk Assessment

- **Risk**: Target chip unresponsiveness if SWCLK frequency changes dynamically.
  - **Mitigation**: Standardize on 100 kHz SWCLK frequency across both bitbanging and PIO modes.

> ## ⛔ SUPERSEDED (2026-08-13)
>
> Superseded by `import-debugprobe-pio-swd`: the old `tap_pio`-based PIO engine described
> here was **replaced wholesale** by the vendored Raspberry Pi **debugprobe PIO transport**
> (`probe_pio_transport.c`), which works (12/12 SWD, 5/5 suite tiers) and needs none of the
> origin/turnaround fixes below. The cross-transaction boundary issue that parked this
> change was resolved by abandoning the engine, not by fixing it. Kept for historical
> record; do not resume.

## PARKED (2026-08-12) — Findings & State Handoff

This change is **parked**. The debugger + flasher stack ships on the bitbang driver first
(`stm32f1-bitbang-debugger` change); the PIO integration resumes here later.

### What was fixed and verified (keep these when resuming)

1. **Read-first ordering** — after line reset / JTAG-to-SWD switch, the first transaction
   MUST be a DPIDR read; ABORT/CTRLSTAT writes only after it succeeds (matches BMP +
   debugprobe). Implemented in `swd_read_idcode()` and the `SWD PIO_IDCODE` handler.
2. **`.origin` mismatch** — `pio_add_program_at_offset()` rejects programs whose declared
   `.origin` differs from the load offset (`PICO_ERROR_BAD_ALIGNMENT`), and `tap_pio_init`
   was ignoring return codes. `check_ack` was declared `.origin 18` but loaded at 20;
   `turnaround_float_to_drive` at 26 vs 28. The SM jumped into a leftover `jmp 20`
   self-loop → parked at pc=20. Fixed by aligning origins (check_ack @20, turnaround @27).
3. **seq_in sampling** — `in pins, 1 side 1` at the rising edge reads a stale value because
   the STM32F1 target updates SWDIO at the rising edge and the PIO input sync lands the
   read ~300ns earlier. Fixed with `nop side 1 [3]` → `in pins, 1 side 0 [2]` (sample
   mid-high-phase, like the bitbang 2µs post-edge sample).
4. **bitbang re-init** — `swd_bitbang_read_idcode()` de-inits the PIO for raw GPIO and
   previously never restored it, breaking every later PIO transfer (~34s IDCODE + degraded
   PIO_IDCODE). Fixed by re-initing (`swd_init()`) at the end.
5. **stale direction cache** — `swd_jtag_to_swd()` / `swd_line_reset_ext()` now reset
   `current_swdio_dir = SWDIO_DIR_FLOAT` at entry; previously a stale `DRIVE` made the
   first seq use `SEQ_OUT` (no pindirs change) while the capture handler had set SWDIO to
   input → request never driven → silent target.

### Verified results

- `SWD PIO_IDCODE` → `OK SWD PIO_IDCODE 0x1BA01477` (instant, repeated).
- `SWD BITBANG` → `ack=1 idcode=0x1BA01477`, and PIO keeps working afterward.
- `SWD IDCODE` → `OK SWD IDCODE 0x1BA01477` in ~1s (was ~34s before fix #4).
- `SWD CAPTURE PIOREAD` → wire shows line reset, switch, request 0xA5, ACK 100,
  data 0x1BA01477, parity OK (212 rising edges, full data phase).
- `SWD CAPTURE PIO` (full) → `res=0 idcode=0x1BA01477`; write data (0x1E) on the wire.
- `SWD TEST` → `OK SWD TEST PASS target=STM32 (Cortex-M3)`.

### Open issue (the reason this is parked)

**Standalone transactions after a completed sequence fail** — target silent:

```
SWD PIO_IDCODE               → OK 0x1BA01477
SWD READ DP 0x0 (bare)       → ERR 5 (raw_ack=0xE0000000 = all-ones, pc=0, txstall=1)
SWD RESET then SWD READ DP 0x0 → OK 0x1BA01477   ← line reset in between fixes it
SWD WRITE DP 0x4 0x50000000  → ERR 5
DEBUG HALT                   → ERR
```

So a transaction only works immediately after a line reset / JTAG-to-SWD switch.

**Leading hypotheses:**
- (a) The `check_ack` mismatch path (`clear_tx_fifo` → `turnaround_float_to_drive`)
  clocks garbage that desyncs the target's SWD state machine between transactions.
- (b) SM parked state / FIFO desync at the transaction boundary (PC parks at START).
- (c) `current_swdio_dir` cache diverging from actual pindirs after the mismatch path.

**Next investigation steps (spike):**
- Add a capture mode that runs read → read and watches the wire at the transaction
  boundary (does the second request go out? is it malformed? does the target see it?).
- Dump SM PC + FIFO state after every transaction (already partially instrumented via
  `swd_diag_state`).
- Try inserting idle cycles between transactions; compare consecutive-transfer behavior
  against MioLink (`references/MioLink/firmware/source/bmp/swdptap.c`).
- Note: writes DO transmit correctly within a burst (0x1E seen on the wire, firmware
  read raw_ack=0x20000000=OK) — the bug is specifically the cross-transaction boundary.

### Files touched by this work (in working tree, uncommitted)

- `firmware/pio/swd.pio` — origins, seq_in sampling, comments
- `firmware/src/programmer/swd.c` — read-first order, bitbang re-init, direction resets
- `firmware/src/programmer/tap_pio.c` — origin load offsets + asserts
- `firmware/src/main.c` — PIO_IDCODE read-first, SWD CAPTURE direction handling,
  SWD READ/WRITE diagnostics

### Parked state

0/6 tasks complete. Re-activate by resuming from task 2.3 (`SWD PIO_IDCODE` verification)
onward after the bitbang driver (`stm32f1-bitbang-debugger`) is shipped; the open issue
above is the entry point for the next PIO debugging session.
