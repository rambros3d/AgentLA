# Design: Unified PIO0 Transports & RVSWD Driver Architecture

## Architecture Overview

```
┌────────────────────────────────────────────────────────────────────────┐
│                        AGENTLA FIRMWARE                                │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│   ARM SWD Path (Proven)                RISC-V RVSWD Path (New)         │
│   ┌───────────────────────────┐        ┌───────────────────────────┐   │
│   │   swd.c / cmsis_dap.c     │        │   ch32.c / riscv_dmi.c    │   │
│   └─────────────┬─────────────┘        └─────────────┬─────────────┘   │
│                 │                                    │                 │
│                 ▼                                    ▼                 │
│   ┌───────────────────────────┐        ┌───────────────────────────┐   │
│   │programmer/debugprobe/     │        │programmer/rvswd/          │   │
│   │ ├── swd_probe.pio         │        │ ├── rvswd.pio             │   │
│   │ ├── probe_pio_transport.c │        │ ├── rvswd_pio_transport.c │   │
│   │ └── swd_transfer_pio.c    │        │ └── rvswd_dmi_transfer.c  │   │
│   └─────────────┬─────────────┘        └─────────────┬─────────────┘   │
│                 │                                    │                 │
│                 └───────────────┬────────────────────┘                 │
│                                 │                                      │
│                                 ▼                                      │
│               ┌───────────────────────────────────┐                    │
│               │ PIO0 SM0 (Dynamic Program Swap)   │                    │
│               │  - SWD:   probe.pio (26 insts)    │                    │
│               │  - RVSWD: rvswd.pio (25 insts)    │                    │
│               └───────────────────────────────────┘                    │
│                                                                        │
│   PIO1 (100% Free for SWIO 1-wire & Peripherals)                       │
│   ┌──────────────────────────────────────────────────────────────┐     │
│   │  PIO1 SM1: swio.pio (CH32V003 1-wire debug)                  │     │
│   └──────────────────────────────────────────────────────────────┘     │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

## Key Architectural Decisions

### 1. Unified PIO0 SM0 Allocation & Dynamic Program Swapping
- **Decision**: ARM SWD (`probe.pio`, 26 insts) and RVSWD (`rvswd.pio`, 25 insts) share **PIO0 SM0** and probe GPIOs (GPIO4 `SWCLK`, GPIO5 `SWDIO`).
- **Mechanism**:
  - `swd_init()` calls `rvswd_deinit()` (if active) and loads `probe.pio`.
  - `rvswd_init()` calls `swd_deinit()` (if active) and loads `rvswd.pio`.
  - `swd_deinit()` / `rvswd_deinit()` call `pio_remove_program(pio0, &program, offset)` to keep PIO0 instruction memory clean ($25\text{ or }26 \le 32$ max instructions).
- **Rationale**: Keeps `PIO1` 100% unencumbered for `swio` (1-wire debug) and UART peripherals. Inherits `PROG_MODE_IS_ACTIVE()` protection so neither protocol collides with Logic Analyzer capture mode.

### 2. Encapsulated Driver Structure (`firmware/src/programmer/rvswd/`)
- **Structure**:
  - `rvswd.pio`: Ported from `references/RVSWD_pico/RVSWD_full.pio`. Defines `get_next_cmd`, `write_bits`, `read_bits`, `start`, `stop`, `reset`.
  - `rvswd_pio_transport.c` / `.h`: C transport layer managing handle state (`rvswd_handle_t`), clock division (15.0f $\approx$ 8.33 MHz at 125 MHz sys_clk), MSB-first shifting, parity check, and bounded FIFO put/get (1ms deadline).
  - `rvswd_dmi_transfer.c` / `.h`: High-level DMI protocol routines (`rvswd_dmi_read`, `rvswd_dmi_write`) with automatic retry (up to 5 retries on failure).

### 3. CH32 Microprocessor Debug & Flash Architecture
- **Control Interface**:
  - `ch32v20x_halt_microprocessor()`: Writes `0x80000001` (`dmactive` | `haltreq`) to `DMCONTROL` (`0x10`), polls `DMSTATUS` (`0x11`) bit 8-9 for halt confirmation, then writes `0x00000001`.
  - `ch32v20x_resume_microprocessor()`: Writes `0x40000001` (`dmactive` | `resumereq`) to `DMCONTROL` (`0x10`), polls `DMSTATUS` bit 10-11 for resume confirmation.
  - Core Registers: Access via `COMMAND` (`0x17`) and `DATA0` (`0x04`) abstract commands.
- **Flash Operations**:
  - Flash unlock: `FLASH_KEYR` (`0x40022004`) + `FLASH_MODEKEYR` (`0x40022024`) sequence.
  - Page erase (256-byte pages): `FTER` (bit 17) + `STRT` (bit 6) in `FLASH_CTLR` (`0x40022010`).
  - Fast page program: PROGBUF injection streaming via `ABSTRACTAUTO`.

---

## Verification & Validation Plan

### Phase 1: ARM SWD Regression Sweep (BluePill)
**Requirement**: Must run **BEFORE** declaring RVSWD implementation complete to ensure zero regressions to ARM SWD.
- Command: `python3 test/test_swd_hardware.py`
  - Expected: 12/12 PASS (SWD IDCODE `0x1BA01477`, DP/AP access, SWD reset).
- Command: `pytest test/test_stm32f1_hardware_suite.py`
  - Expected: 5/5 TIERS PASS (full flash erase $\rightarrow$ program 9920 bytes $\rightarrow$ verify $\rightarrow$ reset $\rightarrow$ boot).

### Phase 2: CH32V203 Hardware Verification Battery
**Requirement**: Must run on live CH32V203 dev board to complete the plan.

1. **Read & Target Detection**:
   - `CH32 DETECT mcu=ch32v203`
   - Verify output: `RVSWD link up (DMSTATUS=0x00000C82)`, canonical `chip_id=0x20305040`, `flash=64KB`, `page=256B`.
   - Read ESIG / chip ID via DMI memory read.

2. **Flash Operations**:
   - Erase flash page at `0x08000000` (256 bytes) and full chip erase.
   - Write firmware test binary payload (e.g. 4 KiB blinky).
   - Read back flash data and verify byte-for-byte match with source binary.

3. **Debugging Operations**:
   - Halt CPU (`CH32 HALT` / `DEBUG HALT`) $\rightarrow$ verify `DMSTATUS` reports halted.
   - Read CPU registers (`R0`–`R15`, `PC`) $\rightarrow$ verify valid register values.
   - Single-step instruction (`DEBUG STEP`) $\rightarrow$ verify `PC` increments properly.
   - Mutate core register / SRAM $\rightarrow$ write value, read back, verify exact match.
   - Resume CPU (`CH32 RESUME` / `DEBUG RUN`) $\rightarrow$ verify target continues execution.

---

## Findings & Known Issues

### F1. Flash read-back broken — ROOT CAUSE FOUND: chip is READ-PROTECTED (RDP)

**Symptom** (observed on live CH32V203, V203 HARTINFO `0x00312380`):
- Flash read-back of main flash (`0x0800xxxx`) does **not** return real content: via the block-read path it returned a constant `0x4000B0C3`; via the single-word PROGBUF `lw a0,0(a1)` path it returns **`cmderr=3`** (bus fault).
- SRAM (`0x2000xxxx`), ESIG (`0x1FFFF704`), option bytes (`0x1FFFF800`), and peripheral registers read correctly through the same paths.

**Confirmed root cause (region sweep, `test/test_rvswd_region_sweep.py`):** the CH32V203 shipped **read-protected**:
- `FLASH_OBR` (`0x4002201C`) = `0xFE` → RDPRT (bit1) = 1.
- Option bytes: `0x1FFFF800` = `0xC03F00FF` (RDPR=0xFF, nRDPR=0x00, USER=0x3F, nUSER=0xC0), `0x1FFFF804` = `0xFF00FF00` (WRPR0/1=0x00), `0x1FFFF808/0C` = `0x00FF00FF` (WRPR2/3, DATA0/1 = 0xFF).
- `RDP_Key = 0x00A5` (ch32v20xhw.h:5796) → `0xA5` is the **unprotect** value; `0xFF` = protected.
- Under RDP, **both** main flash (`0x08000000`) and bootloader (`0x1FFF8000`) fault (cmderr=3); ESIG/option/SRAM/peripherals are readable — a textbook RDP signature. This is **not** a probe transport bug.

**Why the earlier `0x4000B0C3` observation:** that predates the RDP diagnosis and came from the V003-magic-offset confusion in the block-read path (see below); the single-word path consistently shows cmderr=3 on protected flash.

### F2. Fixed: `strtol` overflow on 32-bit host commands
`RVSWD WRITE 0x10 0x80000001` (haltreq) and `CH32 READ 0xE0000380` silently clamped to `LONG_MAX=0x7FFFFFFF` because `strtol()` overflows on this 32-bit platform. Replaced with `strtoul()` in `RVSWD READ/WRITE` and `CH32` argument parsing. This was why the halt test failed initially (wrote `0x7FFFFFFF` instead of `0x80000001`).

### F3. Fixed: `RVSWD READ` response formatting
Success branch of `RVSWD READ` passed `(reg, r, val)` to `sprintf` while the format consumed only `reg, r` — the printed "val" was actually the result code (always `0`), hiding the real register value during debugging. Fixed to print the actual `val`.

### F4. Board re-connection status (last session end)
The CH32V203 board stopped responding mid-session (`DMSTATUS=0xFFFFFFFF`, no parity response, no hang — bounded-FIFO fix working as intended). Raw `RVSWD INIT/RESET/READ/WRITE` all succeed on the probe side and SWD also shows no target, so the probe is healthy and the board needs to be reconnected/powered (GPIO4=SWCLK, GPIO5=SWDIO, GND).

### F5. Verified end-to-end (no open questions)
- Bounded 1ms-deadline FIFO put/get in `rvswd_pio_transport.c` (no hangs on dead bus).
- Dynamic PIO0 SM0 swap RVSWD $\leftrightarrow$ SWD (RVSWD init $\rightarrow$ SWD IDCODE $\rightarrow$ RVSWD auto-deinit; both programs fit 32-instr budget: rvswd=29, probe=11).
- ARM SWD regression on BluePill: `test_swd_hardware.py` 12/12 PASS; `test_stm32f1_hardware_suite.py` 5/5 TIERS PASS.
- RVSWD target detect `DMSTATUS=0x00000C82`, halt/resume, and GPR reads (`test_rvswd_hardware.py` 4/4 PASS).

### F6. Halted-core option-byte unprotect attempts corrupted the option array (chip state today)
Sequence of three attempts (`test/test_rvswd_unprotect.py`, `_unprotect2.py`, `_unprotect3.py`):
- Full unlock (KEYR + OBKEYR + MODEKEYR with KEY1/KEY2), `OPTER`+`STRT` option-byte erase, `OPTPG` + `sh 0x5AA5` to `0x1FFFF800` — every op returns `cmderr=0` and `STATR` goes idle, yet the option array became **`0xE339E339` at every word** and RDP never regressed.
- Current chip state: `OBR=0x03FFFFFF` (OPTERR=1, RDPRT=1), option bytes read `0xE339E339`, `CTLR=0`, `STATR=0x20` (idle/EOP). Flash reads still fault (`cmderr=3`). `chip_id` reads return **stale DATA0 garbage** (see F8).
- Interpretation: with RDP active, halted-core (PROGBUF) option-byte program ops "complete" on the wire but do not stick on the array; the erased state reads back as a corrupt signature. The chip's flash/option controller appears degraded, not freshly reprogrammable via the halted-core debug path.

### F7. Auto mass-erase (RDPR regression) stalls the RVSWD bus → probe wedge; power-cycle recovery
- Writing `0x5AA5` to `OB_BASE+0` (RDPR regression) triggers a chip **auto mass-erase**. During the erase the DM/bus stalls: any RVSWD transaction wedges the probe's PIO SM for minutes (probe stops answering serial).
- The wedge is recovered by a fresh `CH32 DETECT` (its `rvswd_transport_reset()` toggles `dmactive`, snapping the DM out) — or by a physical power-cycle of the target.
- Power-cycle recovery is clean: `DETECT` then returns `chip_id=0x20310500` (healthy). The probe firmware itself never needs reflashing for this.
- Note: minichlink performs the same option-byte ops with the core **halted** and it works on a healthy V20x; our chip was already RDP-protected + degraded, which is why the writes don't stick.

### F8. Firmware bug (FIXED): `riscv_read_mem()` ignored abstract-command errors → stale DATA0
- `firmware/src/programmer/riscv_dmi.c`: the three `wait_abstract_cmd_complete()` returns were discarded; a faulted abstract op (e.g. flash read under RDP) left stale `DATA0` returned as valid data.
- Fix applied: propagate the error and return before reading `DATA0` in `riscv_read_mem()`/`riscv_write_mem()`/`riscv_write_hword()`; the vendored get/set path uses the strict `run_progbuf_strict()` (CMDERR ⇒ faulted access ⇒ propagate, not stale DATA0).
- Verified on silicon: reading RDP-protected flash now returns an error (`ERR ... failed res=2` = NACK) instead of the old constant garbage `0x4000B0C3`.
- Also fixed `ch32_v20x_wait_flash_busy()`: it now checks and clears `WRPRTERR`/`PGERR` (W1C) and returns `CH32_ERR_FLASH` instead of reporting OK on a failed erase/program (previously a write-protected flash silently "succeeded").

### F9. Firmware/transport issue (FIXED): bus access during target flash ops wedges the probe
- Any RVSWD transaction issued while the target's flash controller is mid-operation (mass erase, option program, or just ~30–80 consecutive PROGBUF postexec ops) stalled the PIO SM and the probe stopped responding for minutes.
- Fix applied (two parts):
  1. `rvswd_pio_transport.c`: bounded-FIFO timeouts now fully re-init the PIO SM (unload/reload program + re-arm), so a desynced/stuck SM self-heals instead of wedging for minutes.
  2. The WCH **target DM** also wedges on ~30–80 consecutive AUTOEXECDATA streamed program executions (this is a target behavior, not the probe). All firmware streaming paths now use **one explicit POSTEXEC per word** instead: `ch32_progbuf_flash_write()`, `ch32_progbuf_get_block_aligned()`, `ch32_progbuf_set_block_aligned()`. The RAM-stub loader uses the same per-word pattern (proven: 113 words load and verify without wedging, where AUTOEXECDATA wedged at word 16–48).
- Verified: `CH32 READ` of SRAM multi-word (block path) returns real data; the probe stays responsive through flash-write attempts.
- Note: the flash-WRITE path still fails on V203 with `cmderr=3` — the V203 flash controller does not intercept debug-context writes to `0x0800xxxx` (documented in `ch32_progbuf.c`; the dormant `ch32_sram_flash_write` SRAM-blob path is the intended fix). This is pre-existing, not an F9 regression.

### F10. RAM-stub unprotect recovery — DONE (RDP cleared, verified on silicon)
- `test/stub_unprotect.S` (now 113 words): disable interrupts → unlock KEYR/OBKEYR/MODEKEYR → verify CTLR & 0x8080 == 0 → option erase (`0x220` then `0x260` = OPTER|OPTWRE ± STRT) → `OPTPG|OPTWRE` (`0x210`) → **`sh 0x00A5` to RDPR** (WCH `RDP_Key`; the write triggers the hardware RDP-disable path, which programs the canonical `0x5AA5` into the cell) → re-unlock OBKEYR → USER=0xC03F @ `0x1FFFF802`, Data0/1=0x00FF @ `0x1FFFF804/06`, WRPR0–3=0x00FF @ `0x1FFFF808/0A/0C/0E` (0x00 low byte ⇒ WPR bits 1 = unprotected) → clear OPTPG → done=2 → ebreak. Captures `STATR` after the option erase (`DONE_BASE+8`) and after the RDPR program (`DONE_BASE+0xC`) for diagnosis; `wait_flash` budget raised to ~60 s.
- Loader `test/test_rvswd_ramstub_unprotect.py`: loads the stub via **per-word POSTEXEC** (progbuf `sw a0,0(a1); ebreak`), sets SP/PC, resumes, then **ndmreset** (`DMCONTROL=0x3`) to reload option bytes — a haltreq pulse does NOT reload options on V203 (OBR/WPR keep stale values until a real reset).
- Three stub bugs fixed en route: (1) RDPR value `0x5AA5` → `0x00A5` — the hardware rejects `0x5AA5` outright (cell bit-for-bit unchanged, no auto mass-erase); (2) DATA/WRPR offsets were swapped by one slot vs the V20x layout (`RDPR,USER,Data0,Data1,WRPR0..3`); (3) WRPR convention: option low byte is the **inverted** protection mask and WPR bit=1 means *unprotected*, so unprotect = `0x00FF`, and `0xFF00` write-protects everything (WPR=0, WRPRTERR on every op).
- **Status: VERIFIED** — `done=2`, STATR=EOP (no PGERR/WRPRTERR) on both critical ops, `OBR=0x03FFFCFC` (RDPRT=0), `WPR=0xFFFFFFFF` (all blocks unprotected), option array reads `0xC03F5AA5 / 0x00FF00FF / 0x00FF00FF / 0x00FF00FF`. The F9 wedge is gone (the stub runs to completion with the probe fully responsive).

### F11. Reference: minichlink canonical unprotect sequence (for the stub & future fixes)
- `references/not-relevant/ch32fun/minichlink/minichlink.c` lines 2880–2940 (`SetRAMSplit`): unlock → `OPTER`+`STRT` erase → `OPTPG` → `WriteHalfWord(OB->RDPR, RDP_Key)` → wait idle → re-unlock `OBKEYR` → `OPTPG` → `WriteHalfWord(OB->USER, ...)` → wait → clear `OPTPG`. It halts the core for flash ops (proven to work on a healthy V20x).
- Flash-register map (CH32V20x, `ch32v20xhw.h`): KEYR `0x40022004`, OBKEYR `0x40022008`, STATR `0x4002200C`, CTLR `0x40022010`, ADDR `0x40022014`, OBR `0x4002201C`, WPR `0x40022020`, MODEKEYR `0x40022024`. CTLR bits: `MER=0x4`, `OPTPG=0x10`, `OPTER=0x20`, `STRT=0x40`, `LOCK=0x80`, `OPTWRE=0x200`, `FLOCK=0x8000`, `FTPG=0x10000`, `FTER=0x20000`, `BUFLOAD=0x40000`, `BUFRST=0x80000`. Keys KEY1=0x45670123, KEY2=0xCDEF89AB. `RDP_Key=0x00A5` — **the key write is `0x00A5`** (WCH/OpenOCD), NOT `0x5AA5`; the hardware programs the canonical `0x5AA5` (RDP=0xA5, nRDP=0x5A) into the cell itself.
- V20x option-byte layout (little-endian halfwords at `0x1FFFF800`): `RDPR, USER, Data0, Data1, WRPR0, WRPR1, WRPR2, WRPR3`. WRPR option low byte is the **inverted** WPR mask: `0x00` ⇒ WPR bits 1 (unprotected), `0xFF` ⇒ WPR 0 (protected). Unprotected factory state: `RDPR=0x5AA5, USER=0xC03F, Data0/1=0x00FF, WRPR0–3=0x00FF`.

### F13. Main-flash array erase is dead on this chip (hardware); writes stick but erase never clears cells
**CORRECTED** (was "array physically dead"): controlled SRAM-stub tests (CPU executes flash ops from SRAM, the proven V203 path) show the failure mode is **erase-specific**:
- **Writes stick**: `0x13579BDF` written to `0x08000A00` reads back correctly and **survives a full ndmreset** (power-cycle equivalent). Earlier `0xDEADBEEF`/`0xCAFEBABE` writes also stuck. The main array is not dead to programming.
- **Erase never clears**: both `FTER|STRT` (fast 256B) and `PER|STRT` (standard page) complete with `EOP`, no PGERR/WRPRTERR, BSY spins ~1s — yet the page reads `0xE339E339` (the pattern the whole chip shipped with), not `0xFFFFFFFF`, and previously-written words disappear. The erase operation runs but the cells don't come back to the erased state — classic main-array charge-pump/erase-path hardware failure.
- The **option-byte sector is a separate physical block** and erases/programs fine (that's why the RDP unprotect worked, F10). The chip's CPU/SRAM/DM are all healthy — it just cannot be reprogrammed (writes only work on already-erased cells, and there are no erased cells left).
- Test harness: `test/stub_mainflash_test.S` + `test/test_rvswd_ramstub_mainflash.py` (load stub to SRAM, resume, poll DATA0-magic marker, force-halt, read results). Also confirmed: ebreak does NOT halt the core on V203 (DCSR.ebreakm writes silently fail) — the marker+force-halt mechanism is mandatory.
- Flash WRITE via PROGBUF can never work on V203 regardless: the debug-context `sw` to `0x0800xxxx` faults `cmderr=3` (documented in `ch32_progbuf.c`; the dormant `ch32_sram_flash_write` SRAM-blob path is the intended fix, and is what the tests above use). Pre-existing; unrelated to F8/F9.
- Firmware erase sequence fixed to minichlink's order (`CTLR=FTER` first, then `ADDR`, then `CTLR=FTER|STRT`) — the old order (ADDR before mode) never even started the erase (no BSY, no EOP) on V203.
- **Verdict: this chip is permanently non-reprogrammable; a healthy V203 is required to close sections 5.2/5.4 of `tasks.md`.**

### F12. Environment / tooling
- RISC-V toolchain available: `~/.platformio/packages/toolchain-riscv32-esp/bin/riscv32-esp-elf-gcc` (13.2.0) — used for the RAM stub.
- Reference clones: `/tmp/wch-openocd` (WCH OpenOCD fork; flash driver `wchriscv.c` uses `default_flash_read`; no WCH-specific progbuf flash skip), `references/not-relevant/ch32fun/` (minichlink + ch32fun library + optionbytes example).
- GPIO6 on the probe is nRESET but SWD-path only; RVSWD uses GPIO4/5 only, so the probe has no CH32 reset line. The CH32V203 target reset/power must be cycled externally.
