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
