## Why

AgentLA's CMSIS-DAP engine is fully operational on ARMv8-M targets (STM32H5), but Direct Flash (REST API `/api/v1/programmer/*`) and the embedded GDB Server (TCP :3333) currently fail when interacting with STM32H5 cores. This failure stems from two architectural differences between ARMv7-M (STM32F1/F4) and ARMv8-M (STM32H5):
1. STM32H5 employs a split-bus CoreSight ADIv5 topology where external debug and DBGMCU control reside on AP 0 (`APB-AP`), while system memory, Flash, and the core PPB reside on AP 1 (`AHB-AP`). Currently, GDB server defaults to AP 0 for all memory reads, and flash routines do not properly un-gate low-power debug clocks (`DBGMCU_CONFIG`).
2. Flash reading via the REST API needlessly halts the core; on Cortex-M33, if the core has executed `WFI` without `DBGMCU_CONFIG` clock un-gating, `cortex.halt()` times out and aborts the entire read operation even though AHB-AP can perform non-intrusive live reads without halting.

## What Changes

- Configure STM32H5 `DBGMCU_CONFIG` (enabling `DBG_STOP` and `DBG_STANDBY`) and freeze watchdogs (`DBGMCU_APB1LFREEZE`) upon SWD target attachment.
- Make memory reading in `read_flash_port_internal` non-intrusive (no halting required for AHB-AP memory reads).
- Enhance `CortexM` in `agentla-programmer` to support dual-AP architecture: automatically routing core execution control to the appropriate AP while routing system memory/flash operations to AP 1 on ARMv8-M devices.
- Update `gdb_server.rs` to detect ARMv8-M (STM32H5) targets upon client connection and route GDB memory read/write commands (`m` / `M`) to AP 1.

## Capabilities

### Modified Capabilities
- `rest-target-programmer`: Add non-intrusive live memory reads and DBGMCU low-power un-gating for STM32H5 targets.
- `network-gdb-server`: Add ARMv8-M / Cortex-M33 dual-AP routing to GDB server memory read/write commands.

## Impact

- `crates/agentla-programmer`: `CortexM` dual-AP awareness and AP selection.
- `firmware-rs/src/programmer/mod.rs`: `read_flash_port_internal`, `flash_target_port_global`, `erase_flash_port_global` target configuration.
- `firmware-rs/src/programmer/gdb_server.rs`: Target detection and AP 1 memory mapping for GDB RSP sessions.
- APIs affected: `GET /api/v1/programmer/read`, `POST /api/v1/programmer/flash`, `POST /api/v1/programmer/erase`, GDB TCP :3333.
