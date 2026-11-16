## Context

On ARMv7-M architectures (STM32F1/F4), a single `AHB-AP` (AP 0) bridges all CPU debug registers (`0xE000EDF0`, `DHCSR`), system memory, peripherals, and Flash. On ARMv8-M architectures (STM32H503), the CoreSight topology splits external debug components onto AP 0 (`APB-AP`), while system memory and the core Private Peripheral Bus (PPB) are accessed via AP 1 (`AHB-AP`).

Currently:
1. `gdb_server.rs` creates `CortexM::new(&mut swd)` defaulting to `apsel = 0`, causing GDB memory reads on STM32H5 to target AP 0 and return `E01`.
2. REST flash reads (`/api/v1/programmer/read`) unnecessarily halt the core before reading; on STM32H5 executing `WFI`, the debug clock is disabled unless `DBGMCU_CONFIG` is written, causing halt to timeout and the read to fail.

## Goals / Non-Goals

**Goals:**
- Provide full, non-intrusive memory read capability via `/api/v1/programmer/read` on STM32H5.
- Ensure STM32H5 target halt and flash programming succeed even when user firmware executes `WFI` by configuring `DBGMCU_CONFIG.DBG_STOP` and `DBG_STANDBY`.
- Enable GDB memory inspection (`m<addr>,<len>`) and modification over TCP port 3333 on STM32H5 by auto-selecting AP 1 (`AHB-AP`) with `CSW = 0x23000012`.
- Maintain `#![no_std]`, zero-heap, and strict zero-emoji standards across all crates.

**Non-Goals:**
- Modifying the CMSIS-DAP protocol layer (CMSIS-DAP is already 100% verified and operational).
- Supporting TrustZone Secure-World debugging configurations (defaults to non-secure privileged).

## Decisions

### 1. Non-Intrusive Memory Reading in `read_flash_port_internal`
- *Rationale*: ARM CoreSight `AHB-AP` is a hardware bus master that can access Flash, SRAM, and peripherals asynchronously while the CPU core is running. Removing `cortex.halt()` from `read_flash_port_internal` eliminates unnecessary target execution disruptions and avoids halt timeout failures.
- *Alternatives considered*: Halting under reset; rejected because it interrupts live running target firmware.

### 2. Auto-Configuring `DBGMCU` on STM32H5 Attachment
- *Rationale*: When STM32H5 firmware sleeps with `WFI` or `WFE`, the internal clock to the Cortex-M33 core and debug interface is gated off unless `DBGMCU_CONFIG` (`0x44024004` / `0xE00E4004`) has bits `DBG_STOP` (`1 << 1`) and `DBG_STANDBY` (`1 << 2`) enabled, and watchdogs frozen via `DBGMCU_APB1LFREEZE` (`0x44024008`). Enabling these bits during attach ensures `DHCSR.C_HALT` halts the core immediately.
- *Alternatives considered*: Requiring host scripts to issue reset catches manually; rejected because REST API must be autonomous.

### 3. Target-Aware AP Selection in GDB Server
- *Rationale*: When a GDB client connects to TCP port 3333, `gdb_server.rs` queries DPIDR. If `dpidr == 0x6BA02477` (STM32H5 Arm DPv2) or `0x0BE12477`, it sets `cortex.select_ap(1)` and `cortex.csw = 0x23000012`.
- *Alternatives considered*: Dynamic AP switching per memory address range; unnecessary for STM32H5 because AP 1 provides full memory-mapped access to both system memory and internal PPB.

## Risks / Trade-offs

- **[Risk]** Memory writes during active DMA might observe torn reads if target is not halted.
  → *Mitigation*: Flash programming and erase continue to halt the target core; only read queries allow non-intrusive access.
- **[Risk]** `DBGMCU_CONFIG` write on non-H5 targets if misidentified.
  → *Mitigation*: Gated strictly behind DPIDR verification and DBGMCU IDCODE device ID check (`0x474` for STM32H503).
