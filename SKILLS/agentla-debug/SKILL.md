---
name: agentla-debug
description: Hardware electrical triage, target rail verification, CoreSight DPIDR/CPUID auto-discovery, Access Port (AP) scanning, and in-depth Cortex-M HardFault (CFSR/HFSR/MMFAR) register root-cause analysis.
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  target_architectures: ["ARM Cortex-M0/M0+/M3/M4/M7/M23/M33", "RISC-V", "RP2040", "RP2350", "STM32"]
  tools_required: ["agentla-mcp"]
---

# Hardware Triage & Debug Specialist (`agentla-debug`)

This skill guides AI agents through non-destructive electrical audits, CoreSight silicon detection, and deep register-level post-mortem triage of embedded targets that fail to connect, crash silently, or halt in a HardFault loop.

---

## Triage & Diagnostic Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. ELECTRICAL RAIL & GROUND INTEGRITY                                       │
│    status ──▶ target_diagnose                                               │
│    [Validate: 1.65V ≤ Vtarget ≤ 3.6V, no overcurrent, no pin short-to-GND] │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. CORESIGHT & DP/AP SCANNING                                               │
│    probe_target ──▶ target_diagnose                                         │
│    [Read: DPIDR (SW-DP), CPUID (SCB), AP0 MEM-AP, AP1 CTRL-AP rescue]       │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. HALT & EXECUTION STATE INSPECTION                                        │
│    target_diagnose ──▶ programmer_read                                      │
│    [Read: DHCSR (S_LOCKUP, S_HALT, S_SLEEP), current PC and SP]             │
└───────────────────┬─────────────────────────────────────┬───────────────────┘
                    │ Running / Responsive                │ Faulted / Crashed
                    ▼                                     ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│ 4a. SYSTEM HEALTHY                   │  │ 4b. HARDFAULT ROOT-CAUSE DECODE   │
│     Target operational, vector table │  │     SCB->CFSR, HFSR, MMFAR, BFAR  │
│     valid, peripherals clocked.      │  │     Unwind stacked frame (PC, LR) │
└──────────────────────────────────────┘  └───────────────────────────────────┘
```

---

## Safety Invariants & Diagnostic Rules

> [!CAUTION]
> **Brownout Protection**: Never attempt to drive SWD clock or data lines if `vtarget_mv` is below $1650\text{ mV}$ or floating. Doing so can cause ESD diode conduction, parasitically powering target silicon through SWDIO and inducing latchup.

1. **Passive Probing First**: Always run `status` and `target_diagnose` before attempting writes. Never reset or erase a crashed target before capturing its fault registers!
2. **Lockup Trap Detection**: If `DHCSR.S_LOCKUP == 1`, the processor has double-faulted and halted instruction fetch entirely. Software breakpoints will not trigger; only hardware SWD intervention can inspect the core.
3. **Stacked Frame Validity**: When unwinding the exception stack frame, verify whether the core used MSP (Main Stack Pointer) or PSP (Process Stack Pointer) by inspecting `EXC_RETURN` in the Link Register (`LR & 0x4`).

---

## Step-by-Step Diagnostic Protocol

### Phase 1: Electrical Rail & Continuity Check

1. Call MCP tool `status`:
   - Inspect `vtarget_mv`:
     - If `vtarget_mv < 500`: Target is unpowered. Notify user to connect $3.3\text{ V}$ power or battery.
     - If $500 \le \text{vtarget\_mv} < 1650$: Under-voltage brownout. Target oscillator and flash charge pumps cannot function.
     - If $1650 \le \text{vtarget\_mv} \le 3600$: Safe operating window.
     - If `vtarget_mv > 3600`: **Overvoltage danger!** Halt immediately.
2. Call MCP tool `target_diagnose`:
   - Inspect `pins` section:
     - `swclk`: Must show high impedance or weak pull-down; no short to GND.
     - `swdio`: Must show pull-up (~100kΩ internal).
     - `nrst`: Must show high ($>2.5\text{ V}$) unless reset button is actively held down.

---

### Phase 2: CoreSight Debug Port & Silicon Identification

1. Call MCP tool `probe_target`:
   - Read CoreSight `dpidr`:
     - `0x0BB11477`: Cortex-M0 / M0+ (ARM DPv1)
     - `0x1BA01477`: Cortex-M3 (ARM DPv1)
     - `0x2BA01477`: Cortex-M4 / M7 (ARM DPv1)
     - `0x0BE12477`: Cortex-M23 / M33 (ARM DPv2 / v3)
     - `0x00000000` or `0xFFFFFFFF`: Line disconnected or floating.
2. Read CPUID register (`0xE000ED00`):
   - Part numbers:
     - `0xC20`: Cortex-M0
     - `0xC60`: Cortex-M0+
     - `0xC23`: Cortex-M3
     - `0xC24`: Cortex-M4
     - `0xC27`: Cortex-M7
     - `0xD21`: Cortex-M33
3. Call MCP tool `catalog_lookup` to bind memory boundaries (RAM base, Flash base).

---

### Phase 3: Cortex-M HardFault & Crash Analysis

If a target crashes or becomes unresponsive:

1. Call MCP tool `target_diagnose` to fetch CoreSight registers:
   - **`DHCSR` (Debug Halting Control and Status Register, `0xE000EDF0`)**:
     - Bit 19 (`S_LOCKUP`): Core is in permanent lockup (double fault).
     - Bit 18 (`S_SLEEP`): Core is asleep (`WFI` / `WFE`).
     - Bit 17 (`S_HALT`): Core is halted under debug control.
2. Read **SCB Fault Registers** via `target_diagnose` or `programmer_read`:
   - **CFSR (Configurable Fault Status Register, `0xE000ED28`)**:
     - *UsageFault (bits 31..16)*:
       - `DIVBYZERO` (bit 25): Integer division by zero with `DIV_0_TRP` enabled.
       - `UNALIGNED` (bit 24): Misaligned halfword/word access with `UNALIGN_TRP` enabled.
       - `NOCP` (bit 19): Coprocessor disabled (e.g. FPU instruction executed before enabling CPACR).
       - `INVPC` (bit 18): Invalid exception return code in LR.
       - `INVSTATE` (bit 17): Attempted ARM instruction execution (Thumb bit not set in branched address).
       - `UNDEFINSTR` (bit 16): Unknown opcode executed (memory corruption or bad jump).
     - *BusFault (bits 15..8)*:
       - `BFARVALID` (bit 15): `BFAR` (`0xE000ED38`) holds the exact bus address that triggered fault.
       - `STKERR` (bit 12): Stacking error during exception entry (Stack overflow into guard page).
       - `UNSTKERR` (bit 11): Unstacking error on exception exit.
       - `PRECISERR` (bit 9): Precise data bus fault (e.g., reading/writing unclocked peripheral).
       - `IBUSERR` (bit 8): Instruction bus fault (fetching from unmapped address).
     - *MemManageFault (bits 7..0)*:
       - `MMFARVALID` (bit 7): `MMFAR` (`0xE000ED34`) holds address of MPU permission violation.
       - `DACCVIOL` (bit 1): Data access violation.
       - `IACCVIOL` (bit 0): Instruction fetch from Execute-Never (XN) region.
   - **HFSR (HardFault Status Register, `0xE000ED2C`)**:
     - `FORCED` (bit 30): Configurable fault escalated because handler was disabled or faulted.
     - `VECTTBL` (bit 1): Vector table read fault during interrupt dispatch.

---

### Phase 4: Stack Frame Reconstruction

To locate the exact source instruction that crashed:

1. Identify active Stack Pointer:
   - If stacked return `EXC_RETURN` has bit 2 = 1, read `PSP`; otherwise read `MSP`.
2. Inspect the 8-word hardware-stacked exception frame:
   - `SP + 0x00`: Stacked R0
   - `SP + 0x04`: Stacked R1
   - `SP + 0x08`: Stacked R2
   - `SP + 0x0C`: Stacked R3
   - `SP + 0x10`: Stacked R12
   - `SP + 0x14`: Stacked LR (Caller Return Address)
   - `SP + 0x18`: **Stacked Return PC** (Instruction that caused the fault!)
   - `SP + 0x1C`: Stacked xPSR
3. Format the finding into an actionable root-cause report specifying the function or assembly line.

---

## Output Reporting Standard

```tap
TAP version 13
1..4
ok 1 - Electrical Rail Verification (VTGT = 3298 mV)
ok 2 - CoreSight DPIDR Discovery (0x2BA01477 - ARM Cortex-M4 r0p1)
ok 3 - Execution State Check (DHCSR = 0x00030003: Core Halted)
not ok 4 - HardFault Root-Cause Analysis # USAGEFAULT_INVSTATE
  ---
  cfsr: "0x00020000"
  fault_type: "UsageFault: INVSTATE (Invalid Thumb state bit)"
  stacked_pc: "0x0800045A"
  stacked_lr: "0x08000212"
  remedy: "Function pointer branch address must have bit 0 set to 1 for Thumb execution."
  ...
```
