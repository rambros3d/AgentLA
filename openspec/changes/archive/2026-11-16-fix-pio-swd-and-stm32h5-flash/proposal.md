## Why

AgentLA's PIO SWD driver exhibits a critical sideset misconfiguration (`sideset_count = 2` instead of `1`) that inadvertently drives SWDIO during clock transitions, causing CoreSight write data corruption (`WDATAERR` / `ack=4`), which breaks CMSIS-DAP DAP_Transfer writes, pyocd/probe-rs GDB servers, and target memory writes. 

Analysis of the `/ref/hybridlink` reference implementation reveals two additional protocol vulnerabilities in AgentLA:
1. SWD transactions omit trailing idle cycles when `idle_cycles == 0`. ARM ADIv5 and `hybridlink` mandate at least 2 trailing idle cycles (`write_bits(0, 2)`) to flush CoreSight DP internal write pipelines and prevent bus contention during back-to-back packets.
2. CMSIS-DAP command processing lacks `MATCH_VALUE` and `MATCH_MASK` handling in `DAP_Transfer`, which desynchronizes packet streams when host tools (pyOCD, OpenOCD) poll status registers during flashing and halting.

Furthermore, AgentLA lacks an autonomous flash programming algorithm for STM32H5 series devices (which require 128-bit quad-word programming and 1 KiB TAR boundary management), and the deployed Slot B firmware retains a temporary diagnostic trap in `read_flash_port_internal`. Incorporating these reference insights will establish rock-solid CMSIS-DAP host debugging, GDB server stability, and autonomous direct flash programming over HTTP.

## What Changes

- Fix `sideset_count` in `crates/agentla-programmer/src/pio/swd.rs` from `2` to `1` so the PIO state machine exclusively drives `SWCLK` via sideset while leaving `SWDIO` under `out pins, 1` / `pindirs` control.
- Enforce a minimum of 2 trailing idle cycles (`max(self.idle_cycles, 2)`) on all SWD transactions in `transfer_raw`, matching `hybridlink`'s `swd.c` specification to ensure CoreSight write buffers flush reliably.
- Add `MATCH_VALUE` (`0x10`) and `MATCH_MASK` (`0x20`) support to `handle_transfer` in `crates/agentla-programmer/src/dap.rs` to support host register polling.
- In `cortex_m.rs` and memory access routines, implement 1 KiB boundary chunking (`1024 - (addr & 1023)`) for TAR auto-increment and posted-read `DP_RDBUFF` termination.
- Remove diagnostic intercept (`TargetError::Diag(96)`) in `firmware-rs/src/programmer/mod.rs` and restore clean direct memory read/write over SWD.
- Implement STM32H5 flash programming engine in `crates/agentla-targets/src/engines/stm32.rs` supporting:
  - Flash unlock via `FLASH_NSKEYR` (`0x45670123`, `0xCDEF89AB`)
  - 8 KB sector erase via `FLASH_NSCR` (`SER` + `SNB` + `STRT`)
  - 128-bit (16-byte) quad-word programming loop via `FLASH_NSCR` (`PG`)
  - Error flag checking and clearing via `FLASH_NSCCR`
- Rebuild Slot A release firmware (`--features slot-a`), package OTA image, and deploy to AgentLA at `10.10.10.10`.
- Verify host tools: test CMSIS-DAP v1/v2 packet transfers (`test_cmsis_dap.py`), verify `pyocd gdbserver` and `probe-rs gdb` attach without protocol errors, and verify autonomous direct flash via `POST /api/v1/programmer/flash`.

## Capabilities

### New Capabilities
None.

### Modified Capabilities
- `debugprobe-pio-transport`: Configure PIO state machine with `sideset_count = 1` for `SWCLK` and enforce at least 2 trailing idle cycles after every transaction, guaranteeing clean write transactions and zero `WDATAERR` faults.
- `cmsis-dap`: Support `MATCH_VALUE` and `MATCH_MASK` request flags in `DAP_Transfer` command processing.
- `flash-algorithms`: Add STM32H5 flash programming algorithm supporting 128-bit quad-word write transactions, 8 KB sector erase, 1 KiB TAR boundary chunking, and non-secure flash control register mapping at `0x40022000`.

## Impact

- **Affected Crates**:
  - `crates/agentla-programmer`: `pio/swd.rs` (pinctrl sideset count, idle cycles), `dap.rs` (match value/mask).
  - `crates/agentla-targets`: `engines/stm32.rs` (STM32H5 flash driver and sector layout).
  - `firmware-rs`: `src/programmer/mod.rs` (clean up `read_flash_port_internal` diagnostic stub).
- **APIs**:
  - CMSIS-DAP USB interface (`1209:414d`): write transfers succeed with `ack=1` (OK) and support value matching.
  - REST endpoints: `GET /api/v1/programmer/read` and `POST /api/v1/programmer/flash` work autonomously for STM32H5 targets.
- **Protocols & Tools**:
  - GDB servers (`pyocd`, `probe-rs`) can connect, halt, resume, and single-step STM32H503 targets.
