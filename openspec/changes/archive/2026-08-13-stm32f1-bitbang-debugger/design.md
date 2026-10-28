> ## ⛔ SUPERSEDED (2026-08-13)
>
> This change is **superseded by `import-debugprobe-pio-swd`**. The bitbang plan was
> abandoned: the bitbang transport was removed from the tree (`swd_bb_transfer` deleted,
> `prog_driver_t` reduced to `PROG_DRIVER_PIO` only) and the debugger/flasher now ships on
> the vendored Raspberry Pi **debugprobe PIO transport** (`probe_pio_transport.c`), which
> resolved the CTRL/STAT FAULT blocker (sticky WDATAERR artifact) and passes the full
> hardware battery (12/12 SWD, 5/5 suite tiers). Kept for historical record; do not resume.

## Technical Approach

### 1. Runtime driver selection (programmer layer)

A single global mode shared by all three transports:

```c
typedef enum { PROG_DRIVER_BITBANG = 0, PROG_DRIVER_PIO } prog_driver_t;
extern prog_driver_t prog_driver;          /* default PROG_DRIVER_BITBANG */
```

CLI: `PROG DRIVER BITBANG` / `PROG DRIVER PIO` → sets `prog_driver` and (re)enters the mode's pin ownership. No rebuild needed to A/B. `SWD CAPTURE` forces PIO mode for its window, then restores the selected driver.

### 2. Bitbang mode lifecycle (pin ownership)

All three transports share GPIO4 (SWCLK) and GPIO5 (SWDIO/SWIO_DATA); nRESET is GPIO6. Bitbang mode therefore owns one pin pool:

```
enter_bitbang_mode():
    disable SWD PIO0 SM0, SWIO PIO1 SM1, RVSWD PIO1 SM2
    deinit GPIO4/5/6 to SIO
    configure: SWCLK output (12mA fast), SWDIO output+input w/ pull-up (12mA fast), nRESET output
    prog_driver = BITBANG

exit_bitbang_mode() → re-run the engine init for the PIO backend (swd_init/swio_init/rvswd_init)
```

Enter once per programmer session; do NOT deinit/reinit per transaction (a 1KB flash write is ~300ms at 10µs/bit; per-transaction reinit would dominate).

### 3. Bitbang SWD transfer (swd_bb_transfer)

Mirror the timing proven in `swd_bitbang_read_idcode()` — 5µs half-period clocks, ACK/data sampled ~2µs after the rising edge:

```
swd_bb_transfer(request, rnw, *data):
  if dir == FLOAT: drive SWDIO (turnaround entry)
  write 8 request bits        { set SWDIO; SWCLK high; 5µs; SWCLK low; 5µs }
  release SWDIO (input);      one turnaround cycle with SWCLK low
  read 3 ACK bits             { SWCLK high; delay 2µs; sample SWDIO; SWCLK low; 5µs }
  if rnw: read 32 data + 1 parity (same 2µs sampling); verify parity
  else:   write 32 data + 1 parity; data stable BEFORE rising edge, hold after
  drive SWDIO; 8 idle cycles  (0)
  return ACK → swd_read_dp/write_dp map to OK/WAIT/FAULT/PROTOCOL
```

Key protocol rules preserved:
- **Read-first ordering**: after line reset / JTAG-to-SWD switch the first transaction MUST be a DPIDR read; ABORT/CTRLSTAT writes only after it succeeds.
- Request parity computed via `swd_build_request()` (unchanged).
- WAIT retry: on `ACK_WAIT`, re-issue the transaction (bounded retries), do NOT change clock mid-sequence.

### 4. Fast bitbang connect (swd_read_idcode restructure)

```
line_reset (bitbang) → JTAG-to-SWD switch (0xE79E) → post-switch reset
→ read DPIDR (bitbang, first) → on OK: write ABORT 0x1E + CTRLSTAT 0x50000000
→ optional nRESET pulse + dormant-to-SWD only on failure
```

Removes the PIO 30-retry × clock-stepping loop (root cause of the observed ~34s `SWD IDCODE`); IDCODE reads become milliseconds.

### 5. Bitbang SWIO (WCH CH32 1-wire, GPIO5)

Mirror `swio.pio` framing with GPIO timing (~2.5MHz PIO → bitbang ~250kHz):
`begin (1 low) → 8 data bits LSB-first → 8 complement bits → end (release high)`.
Reads: host sends begin+header, releases, samples data bits mid-bit. `swio_write_byte` / `swio_read_byte` / `swio_transfer` dispatch to the bitbang backend.

### 6. Bitbang RVSWD (WCH RISC-V, GPIO4/5)

Mirror `rvswd.pio` bit framing (SWCLK/SWDIO with RVSWD start/ACK/parity semantics). `rvswd_*` low-level calls dispatch to the bitbang backend. `riscv_dmi.c` / `ch32_progbuf.c` sit above and are unchanged.

### 7. Test strategy

- STM32F1: `test/test_stm32f1_hardware_suite.py` 5 tiers (transport, CPU control, AHB/chip-ID, flash, verify) + `test/test_swd_hardware.py`.
- CH32: `test/test_ch32v003.py`, `test/test_ch32v203.py`, `test/test_ch32v203_hardware_verification.py`, blink tests.
- CMSIS-DAP: probe over HID (`cmsis_dap.c` routes through the swd API — verify a transfer).
- Each tier runs under the bitbang default; PIO mode re-verified only via `SWD CAPTURE`/`SWD PIO_IDCODE`.

## Risk Assessment

- **Risk**: Write-path timing (data setup before rising edge) untested in bitbang on this target.
  - **Mitigation**: Reuse the proven 5µs half-period + 2µs sampling; validate with the DP/AP write tiers first.
- **Risk**: Bitbang throughput on large flash writes.
  - **Mitigation**: 10µs/bit ⇒ ~300ms per 1KB page; acceptable; keep PIO for future speedup.
- **Risk**: CH32/RV targets have no bitbang reference — framing is transcribed from PIO programs.
  - **Mitigation**: Verify against existing CH32/RV hardware tests before declaring done.
- **Risk**: PIO engines drift while parked.
  - **Mitigation**: PIO code frozen; `SWD CAPTURE`/`SWD PIO_IDCODE` kept as smoke tests; findings captured in `stm32f1-pure-pio-swd`.

---

## Session State & Root-Cause Handoff (2026-08-12)

Status: tasks 1.1-1.3 and 2.1-2.3 are **implemented and on hardware**; task 2.4 is
**BLOCKED** on a deterministic CTRL/STAT write FAULT (details below). Tasks 3-6 are
blocked behind 2.4 because the debugger/flasher stacks need working DP/AP writes.

### Implemented this session (bitbang transport)

- `prog_driver` global + `PROG DRIVER BITBANG|PIO` CLI (`cmd_prog_driver`, dispatched in
  `main.c`); default `PROG_DRIVER_BITBANG`.
- Bitbang pin ownership via `swd_bb_gpio_init()` (GPIO4 SWCLK / GPIO5 SWDIO / GPIO6 nRESET,
  12mA fast slew, SWDIO pull-up), entered once per session from `swd_init()`.
- `swd_bb_transfer()` — full read **and write** transactions (request → turnaround → ACK →
  data/parity → idle, 5µs half-period, 2µs post-edge sampling) with parity verification and
  **post-non-OK-ACK line cleanup** (drive SWDIO low + 8 idle clocks; without it the target
  stays mid-transaction and the next request is answered with a silent all-ones ACK).
- Driver dispatch in `swd_transfer()`, `swd_write_bits()`, `swd_read_bits()`,
  `swd_line_reset_ext()`, `swd_jtag_to_swd()`, `swd_write_dp_no_check()`.
- `swd_read_idcode()` bitbang-first fast connect: line reset → switch → **DPIDR read first**
  → ABORT/CTRLSTAT writes after; the slow PIO 30-retry × clock-step loop is removed for
  bitbang (IDCODE reads are now milliseconds).

### Verified on hardware (STM32F103 Blue Pill, bitbang default)

- `SWD IDCODE` → fast, `0x1BA01477`.
- `SWD DBLREAD` (line reset → 2× DPIDR reads) → both ACK OK, IDCODE stable.
- ABORT write (`0x81` / data `0x1E`) → ACK OK.
- Wire-level capture (PIO0 scope samples GPIO4/5 while bitbang drives them — no loopback
  jumpers needed): line reset, JTAG-to-SWD switch, DPIDR request `0xA5`, ACK OK, read data,
  ABORT request `0x81` + data `0x1E` + parity all **correct on the wire**.

### The blocker (deterministic, reproduced many times)

- The **first CTRL/STAT write after any fresh connect gets a genuine FAULT ACK (0,0,1) on the
  wire** — request `0xA9` (addr 0x4, parity 1) is correctly encoded; verified by scope capture,
  not just host-side decode.
- Invariant to the write data value (`0x50000000`, `0x1BA01477`, ...) and invariant to the
  ACK→data turnaround hold (WRALIGN sweep of 0-4 quiet bit periods produced identical FAULTs).
- `swd_read_idcode()`'s bitbang branch ignores the ABORT/CTRLSTAT write ACKs, so the failed
  power-up leaves the DP un-powered; every later non-DPIDR/non-ABORT transaction then fails
  with a silent all-ones ACK (`bb_ack=7`) — the original symptom.

### Theories ruled out

- ACK→write-data turnaround timing (WRALIGN sweep: identical behavior at 0-4 bit periods).
- Request layout with parity duplicated in bit 6 — broke DPIDR reads; STM32F1 uses the
  stop/park layout (bit6=0, bit7=1); **reverted**.
- Write-data phase corruption: scope shows `0x1E` + correct parity on the wire and the ABORT
  ACK is OK (note: ACK precedes the data phase, so a parity error on ABORT's own data would
  set WDATAERR and only FAULT the *next* transaction).

### Debugprobe sanity check (2026-08-12) — independent confirmation

To settle "is the FAULT our bug or the silicon's?", Raspberry Pi's official **debugprobe**
CMSIS-DAP firmware was built for the Pico 2 (RP2350) probe and driven against the same Blue
Pill with pyOCD 0.44:

- **Build**: `cmake -DDEBUG_ON_PICO=1 -DPICO_BOARD=pico2 -DPICO_SDK_PATH=/home/sun/pico-sdk`
  (SDK 2.1.1). Two vendored-source fixes were needed: the repo's FreeRTOS submodule was
  uninitialized (needed `git submodule update --init` to obtain the `RP2350_ARM_NTZ` port), and
  the stale `pico_usb_reset` / `pico/usb_reset.h` / `tusb_reset_interface_driver` SDK API was
  removed upstream — the reset interface (a Windows BOOTSEL/WINUSB convenience) was stripped
  from `usb_descriptors.c` / `tusb_edpt_handler.c` and the CMake link list.
- **Pins**: `board_pico_config.h` moved SWD to GPIO4/5 and nRESET to GPIO6 (UART bridge to
  GPIO0/1) to match the Blue Pill wiring.
- **Connect — PASS**: pyOCD ran the full standard sequence (line reset → JTAG-to-SWD switch →
  DPIDR read → ABORT → CTRL/STAT power-up write → poll) on our Blue Pill: `DPIDR 0x1BA01477`,
  `CTRL/STAT 0xF0000F40` (both power-up ACK bits set), full AHB-AP access, halt/resume.
  **The STM32F1 SW-DP accepts the CTRL/STAT power-up write from a fresh post-reset state.**
- **Feature sweep — 13/13**: core registers, memory R/W, live peripheral probing (PC13 LED
  toggle observed while the target ran), single-step, hardware breakpoint, register write,
  resume, and the CMSIS-DAP v1 (HID) transport. nRESET (GPIO6) is not wired on this rig
  (an optional line per `docs/pin-allocation.md`).
- **Heartbeat end-to-end**: `test/bluepill_blink` heartbeat firmware flashed via pyOCD
  (erase/program/verify, 11.6 kB/s) and verified executing (halted inside the main loop,
  PC13 configured as output). Note: the Blue Pill's **BOOT0 jumper is set to "1"**, so it
  boots the ROM bootloader; user flash runs only under debugger control until BOOT0 is
  moved to "0".

**Conclusion**: the target, wiring, and the standard ADIv5 sequence are all proven fine. Our
 deterministic CTRL/STAT FAULT is **our firmware's bug** — a sticky flag set by an earlier
 phase of OUR bitbang sequence (WDATAERR on the ABORT write's data phase, or STICKYORUN),
 since debugprobe performs the identical sequence successfully. pyOCD also confirms the
 working power-up value is **`0x30000000`** (bit28|bit29), not `0x50000000`.

### Open theories / next steps (updated after the debugprobe sanity check)

1. ~~Power-up value~~ **RESOLVED by pyOCD**: `0x30000000` (bit28|bit29) is the working
   power-up value — switch the `swd_read_idcode()` / `swd_write_dp_no_check()` call sites from
   `0x50000000` to `0x30000000`. (The FAULT was value-independent, so the value is not the
   FAULT trigger, but the fix is still required for correct power-up.) The planned `CTRLSWEEP`
   diagnostic is superseded.
2. **Sticky-flag path — CONFIRMED direction**: the target accepts the standard sequence, so
   OUR sequence sets a sticky flag. Prime suspects: WDATAERR from the ABORT write's data phase
   (ACK precedes data; a wire-level parity error would FAULT exactly the next transaction) or
   STICKYORUN from the DPIDR read. Next: edge-accurately re-decode the ABORT write's parity bit
   from the existing scope capture, and/or diff our per-phase wire timing against debugprobe's
   `SWD_Transfer` (`references/debugprobe/src/sw_dp_pio.c`).
3. ~~Sequence re-ordering~~ **Not needed**: the standard order demonstrably works on this
   SW-DP.

### Diagnostics added (main.c / swd.c / swd.h)

`IDCODEACK`, `CONNECTSTEPS`, `DBLREAD`, `WRITETEST`, `WSEQ`, `WRALIGN`
(`swd_bb_write_turn_clocks`), `SCOPE` / `SCOPEAB` (PIO0 wire capture while bitbang drives
GPIO4/5), and `swd_debug_bb_*` stats. These are debug-only and safe to keep while 2.4 is open.
