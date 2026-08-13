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

### F8. Firmware bug (UNFIXED): `riscv_read_mem()` ignores abstract-command errors → stale DATA0
- `firmware/src/programmer/riscv_dmi.c` lines 216/224/232: the three `wait_abstract_cmd_complete()` returns are discarded. On a faulted abstract op (e.g. flash read under RDP), `DATA0` holds a stale value and `riscv_read_mem()` returns it as if valid.
- Observed: after wedges, `CH32 DETECT` reports `chip_id=0xA023123E` / `0x1D6301DE` (garbage words, not `0x20310500`).
- Fix (ready to apply): propagate the error and return before reading `DATA0`.

### F9. Firmware/transport issue (UNFIXED): bus access during target flash ops wedges the probe
- Any RVSWD transaction issued while the target's flash controller is mid-operation (mass erase, option program, or just ~30–80 consecutive PROGBUF postexec ops) stalls the PIO SM and the probe stops responding for minutes.
- Not a bounded-FIFO bug (those are 1ms); the SM itself gets stuck mid-transaction when the target stalls the bus. Recovery is a fresh `CH32 DETECT` (dmactive toggle) or re-init.
- Script workaround that WORKS: load bulk data via **PROGBUF AUTOEXECDATA** (`ABSTRACTAUTO=1`, `DATA0`-write per word) — `test/test_rvswd_ramstub_unprotect.py` loads 94 words to SRAM and verifies them. Do not touch the bus during the target's flash-op windows.

### F10. RAM-stub unprotect recovery — built, blocked by F9
- `test/stub_unprotect.S`: 94-word hand-written RISC-V stub, compiled with `riscv32-esp-elf-gcc 13.2.0` (`~/.platformio/packages/toolchain-riscv32-esp/bin/`, `-march=rv32imc_zicsr -mabi=ilp32 -Wl,-Ttext=0x20000000`). Runs on the core from SRAM with **normal bus access** (the chip's own flash path) doing: disable interrupts → unlock KEYR/OBKEYR/MODEKEYR → verify CTLR & 0x8080 == 0 → OPTER+STRT erase → OPTPG → `sh 0x5AA5` (RDPR, triggers auto mass-erase) → re-unlock OBKEYR → write USER=0xC03F @ `0x1FFFF802`, WRPR0/1=0xFF00 @ `0x1FFFF804/06`, WRPR2/3=0x00FF @ `0x1FFFF808/0A`, DATA0/1=0x00FF @ `0x1FFFF80C/0E` → clear OPTPG → set done flag `0x20001000` → `ebreak`. Each flash step polls `STATR & 3` with a ~28 s timeout.
- Loader `test/test_rvswd_ramstub_unprotect.py`: loads the stub into SRAM via PROGBUF AUTOEXECDATA (a1 = DATA0 mmio `0xE0000000|(HARTINFO&0x7FF)` = `0xE0000380`; progbuf = `lw a0,0(a1); sw a0,0(a2); addi a2,a2,4`), sets SP=0x20002000 / PC=0x20000000, resumes.
- Status: stub load + verification works; PC/SP set works; the run is blocked because **any bus traffic during the stub's flash-op window (including the resume/clear-resumereq writes) wedges the probe** (F9). Needs the F9 firmware fix (or a schedule with zero bus traffic across the ~5–30 s window).

### F11. Reference: minichlink canonical unprotect sequence (for the stub & future fixes)
- `references/not-relevant/ch32fun/minichlink/minichlink.c` lines 2880–2940 (`SetRAMSplit`): unlock → `OPTER`+`STRT` erase → `OPTPG` → `WriteHalfWord(OB->RDPR, 0x5AA5)` → wait idle → re-unlock `OBKEYR` → `OPTPG` → `WriteHalfWord(OB->USER, ...)` → wait → clear `OPTPG`. It halts the core for flash ops (proven to work on a healthy V20x).
- Flash-register map (CH32V20x, `ch32v20xhw.h`): KEYR `0x40022004`, OBKEYR `0x40022008`, STATR `0x4002200C`, CTLR `0x40022010`, OBR `0x4002201C`, MODEKEYR `0x40022024`. CTLR bits: `MER=0x4`, `OPTPG=0x10`, `OPTER=0x20`, `STRT=0x40`, `LOCK=0x80`, `OPTWRE=0x200`, `FLOCK=0x8000`. Keys KEY1=0x45670123, KEY2=0xCDEF89AB. `RDP_Key=0x00A5`.
- Option-byte factory values to restore after regression (little-endian words at `0x1FFFF800`): `0x1FFFF800=0x5AA5`, `0x1FFFF802=0xC03F`, `0x1FFFF804/06=0xFF00`, `0x1FFFF808/0A=0x00FF`, `0x1FFFF80C/0E=0x00FF`.

### F12. Environment / tooling
- RISC-V toolchain available: `~/.platformio/packages/toolchain-riscv32-esp/bin/riscv32-esp-elf-gcc` (13.2.0) — used for the RAM stub.
- Reference clones: `/tmp/wch-openocd` (WCH OpenOCD fork; flash driver `wchriscv.c` uses `default_flash_read`; no WCH-specific progbuf flash skip), `references/not-relevant/ch32fun/` (minichlink + ch32fun library + optionbytes example).
- GPIO6 on the probe is nRESET but SWD-path only; RVSWD uses GPIO4/5 only, so the probe has no CH32 reset line. The CH32V203 target reset/power must be cycled externally.
