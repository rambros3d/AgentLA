## Context

AgentLA exposes 29 MCP tools covering the physical capabilities of a digital multimeter, logic analyzer, SWD flasher, protocol adapter, and CoreSight register inspector. However, LLM agents have high variance when coordinating these tools. For example:
- Attempting to flash before checking if the target supply rail is powered, causing silent SWD timeouts.
- Resetting the target and immediately polling UART without allowing a 10–50 ms boot settling window.
- Terminating tests with a generic "timeout" error when a target crashes, failing to call `target_diagnose` to inspect CoreSight fault registers (CFSR, HFSR, BFAR).

The `mcu-autonomous-testing` skill codifies a robust closed-loop testing protocol in `.agent/skills/mcu-autonomous-testing/SKILL.md` that guides any AI agent step-by-step through silicon discovery, health audit, flashing, stimulation, and root-cause fault triage.

## Goals / Non-Goals

**Goals:**
- Provide a universal, cross-MCU testing playbook covering ARM Cortex-M (STM32, RP2040, RP2350, nRF) and RISC-V (CH32V, RP2350 Hazard3).
- Implement a 5-phase closed loop:
  1. Auto-Discovery & Health Audit (`status`, `probe_target`, `target_diagnose`, `catalog_lookup`)
  2. Flash Vector Table Pre-Check & Programming (`inspect_firmware`, `programmer_flash`, `programmer_reset`)
  3. Boot Settling & Console Verification
  4. Multi-Protocol Interactive Stimulus (`uart_command`, `can_send` / `can_sniff`, `i2c_scan`, `spi_transfer`)
  5. Automatic HardFault Triage on failure (`target_diagnose`)
- Define explicit safety gates preventing electrical damage or flash corruption during brownout.
- Standardize human- and machine-readable test verdicts (TAP format and structured Markdown tables).

**Non-Goals:**
- Creating custom hardware drivers or changing the 150 MHz firmware core (uses existing MCP tool suite).
- Replacing hardware-in-the-loop (HIL) automated test racks; this skill empowers AI agents to perform autonomous pair-programming verification on developer benches.

## Decisions

### 1. Skill Architecture: Native Markdown Skill vs External Daemon
- **Decision**: Define the skill as a standard Antigravity/Agent markdown skill in `.agent/skills/mcu-autonomous-testing/SKILL.md` with accompanying Python validation suites.
- **Rationale**: Agents consume markdown instructions natively without requiring an external runner or background daemon. The skill leverages standard MCP tool calling semantics.
- **Alternative considered**: Standalone Python CLI test runner. Rejected because it bypasses agent reasoning and cannot adapt dynamically to unexpected device responses.

### 2. Auto-Discovery Hierarchy
- **Decision**: Dynamically discover target silicon without requiring user-specified chip models:
  1. Call `status` to inspect $V_{target}$ rail voltage.
  2. Call `probe_target` to read CoreSight DPIDR/CPUID or RISC-V DMSTATUS.
  3. Call `catalog_lookup` with the detected DevID to look up physical flash base and SRAM bounds.
- **Rationale**: Enables truly universal behavior across any bench setup without brittle hardcoded board manifests.

### 3. Fail-Safe Closed-Loop Triage Tree
- **Decision**: Mandatory HardFault investigation on any stimulus timeout:
  - If a UART command, I2C transaction, or CAN message times out or returns unexpected data, the agent MUST immediately invoke `target_diagnose`.
  - The agent inspects `CFSR` (UsageFault/BusFault/MemManage), `HFSR` (HardFault), and `BFAR` (BusFault Address).
  - The agent synthesizes root-cause triage (e.g. "Target crashed at PC 0x08000244 due to unaligned memory access in driver initialization").
- **Rationale**: Turns opaque timeouts into actionable engineering root causes.

```
                    ┌────────────────────────────┐
                    │  Stimulus Timeout / Error  │
                    └─────────────┬──────────────┘
                                  │
                                  ▼
                    ┌────────────────────────────┐
                    │   Call target_diagnose     │
                    └─────────────┬──────────────┘
                                  │
         ┌────────────────────────┼────────────────────────┐
         ▼                        ▼                        ▼
  [DHCSR Lockup=1]       [CFSR != 0 or HFSR != 0]     [Core Running / No Fault]
         │                        │                        │
         ▼                        ▼                        ▼
  CPU Double-Fault       Usage/Bus/Mem Fault         Peripheral Stall or
  Lockup detected        Root Cause via BFAR/CFSR    Baud Mismatch
```

## Risks / Trade-offs

- **[Risk] Target MCU locked in Readout Protection (RDP Level 1 or APPROTECT)**
  - *Mitigation*: The skill detects RDP level during Phase 1 (`inspect_firmware`). If locked, it prompts the agent/user and offers `programmer_unbrick` to mass-erase before failing.
- **[Risk] Power rail brownout ($V_{target} < 2.7\text{V}$)**
  - *Mitigation*: Hard abort before flashing if $V_{target} < 2.7\text{V}$ to prevent sector corruption.
- **[Risk] Slow bootloaders (>50 ms) causing premature timeouts**
  - *Mitigation*: Mandate minimum 100 ms settling delay after `programmer_reset` before issuing first stimulus.
