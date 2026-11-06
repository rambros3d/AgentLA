## Context

AgentLA runs an on-chip Model Context Protocol (MCP) JSON-RPC 2.0 server directly on the RP2350 microcontroller (`http://10.10.10.10/mcp`), enabling autonomous AI coding agents (such as Claude, DeepMind, or automated CI systems) to control hardware tools. While basic primitives exist for reading raw pins or issuing flash reads, an agent diagnosing a non-booting embedded board currently has to execute 15+ manual queries and decode raw bitmasks.

To make AgentLA a true autonomous hardware assistant, it requires **high-level, composite diagnostic tools** that execute multi-step physical, architectural, and bus diagnostics on-chip and return synthesized root-cause insights in structured JSON.

---

## Goals / Non-Goals

### Goals
- **One-Shot Target Diagnosis (`mcp_target_diagnose`)**: Check target supply rail voltage, test pin electrical integrity (driven vs. pulled-up vs. floating vs. shorted), identify CoreSight DPIDR/CPUID, and diagnose CPU execution state (including HardFault register root-cause analysis) in a single API call.
- **Universal Architecture Compatibility**: Use universal ARM CoreSight System Control Block (SCB) registers (`0xE000ED00`–`0xE000EDF0`) mandated across all ARM Cortex-M microcontrollers (STM32, NXP, Nordic, RP2040, SAM) and standard RISC-V Debug Module 0.13 registers.
- **Dynamic Memory Bounds Resolution**: Validate vector table Stack Pointers and Reset Vectors dynamically against LittleFS `AlgoBlobHeader` metadata (`/system/algo/<chip>.bin`) with zero hardcoded chip definitions in probe ROM.
- **Autonomous Bus Sniffing (`mcp_sniff_bus`)**: Capture an arbitrary window (50 ms – 1000 ms) of I2C/SPI bus activity and feed samples into in-firmware decoders to produce structured transaction lists, slave address inventories, and error logs.

### Non-Goals
- Proprietary vendor-specific trace formats (ETM/ITM) in this phase.
- Continuous multi-hour packet sniffer logging (handled by the Logic Analyzer streaming engine).

---

## Decisions

### 1. Three-Tier Diagnostic Abstraction
* **Decision**: Implement a universal diagnostic engine in `firmware-rs/src/programmer/target/diagnostic.rs`:
  * **Tier 1 (Electrical Layer)**: Uses RP2350 SAR ADC to measure $V_{\text{target}}$ and asserts internal weak pull-downs on `SWCLK`, `SWDIO`, and `nRST` to classify pin states (Hard Driven High, Pulled Up, Floating, or Ground Short).
  * **Tier 2 (Architectural CoreSight SCB)**: Reads mandated standard registers:
    - `DHCSR` (`0xE000EDF0`): Core running vs. halted vs. reset
    - `CFSR` (`0xE000ED28`): Identifies DivByZero, Unaligned Load, Precise BusFault
    - `HFSR` (`0xE000ED2C`): Identifies vector table read fault or escalated fault
    - `BFAR` (`0xE000ED38`): Captures the exact invalid physical address accessed
    - `PC` & `LR`: Captures faulting instruction pointer and exception frame return code
  * **Tier 3 (RISC-V Fallback)**: For QingKe RISC-V cores, queries `DMSTATUS` (`0x11`) and CSRs (`mcause`, `mepc`, `mtval`).
* **Rationale**: This layered model works across all chips from all vendors without needing custom firmware drivers per part number.

### 2. LittleFS-Driven Firmware Inspector
* **Decision**: In `mcp_inspect_firmware`, read the initial vector table words (`0x08000000` / `0x00000000`):
  * Check 1: Initial Stack Pointer within valid SRAM range. SRAM boundaries are resolved from the loaded LittleFS `AlgoBlobHeader` (`ram_base` and `ram_size`). If no blob is loaded, falls back to the universal ARM SRAM window (`0x20000000`–`0x20080000`).
  * Check 2: Reset Vector Thumb bit invariant. ARM Cortex-M mandates bit 0 = 1. If `Reset_Vector & 1 == 0`, flags an instant fatal configuration bug.
  * Check 3: Option byte RDP check to detect silicon lockouts.
* **Rationale**: Allows AI agents to catch invalid linker scripts and wrong memory layouts immediately after flashing.

### 3. Integrated Bus Sniffer Engine
* **Decision**: `mcp_sniff_bus` claims the pin arbiter for the requested duration, triggers a single-shot DMA capture on the target lines (e.g. 500 ms at 1 MHz), feeds the sample buffer into `firmware-rs/src/decoder.rs`, and builds a structured JSON array of decoded transactions (address, R/W, payload, ACK/NACK).
* **Rationale**: Reuses the high-speed PIO capture engine and existing in-firmware decoders without host-side dependencies.

---

## Risks / Trade-offs

| Risk | Mitigation |
| :--- | :--- |
| **Pin perturbation disrupts active target bus** | Weak pull-down test on SWD/nRST lines is executed for < 50 µs only during diagnostic initiation, well within standard signal slew tolerances. |
| **Target in DeepSleep (SWD clock gated off)** | Diagnostic detects SWD attach timeout, reports `TARGET_IN_SLEEP_OR_UNPOWERED`, and recommends connect-under-reset. |
| **High I2C transaction rate exceeds MCP payload** | JSON output caps transactions at the first 32 observed events while reporting total transaction counts. |
