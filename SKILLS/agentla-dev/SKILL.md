---
name: agentla-dev
description: Autonomous firmware developer and debugger that guides user wiring, synthesizes target firmware, flashes, debugs, and physically verifies signal behavior closed-loop using AgentLA logic analyzer and bus instruments until requirements are met.
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  target_architectures: ["ARM Cortex-M", "RISC-V", "RP2040", "RP2350", "STM32", "ESP32", "nRF52", "CH32V"]
  tools_required: ["agentla-mcp"]
---

# Autonomous Firmware Developer & Verification Specialist (`agentla-dev`)

This skill empowers an AI agent to operate as an autonomous embedded firmware engineer. It bridges software synthesis and physical reality by guiding the user through hardware wiring, generating target code, flashing target silicon, instrumenting pins via the AgentLA Pin Arbiter, and physically verifying signal outputs (frequency, duty cycle, protocol frames) in a self-healing closed loop.

---

## Autonomous Closed-Loop Developer Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. REQUIREMENTS & PINOUT CONSULTATION                                       │
│    - Parse target MCU part number, clock speeds, and required peripherals.   │
│    - Map target signals to AgentLA SWD and Logic Capture channels (D0..D7). │
│    - Output crisp, unambiguous wiring instructions to the user.             │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. PRE-FLIGHT ELECTRICAL & CONTINUITY AUDIT                                 │
│    status ──▶ target_diagnose ──▶ catalog_lookup                            │
│    [Verify: Vtarget ≥ 2.7V, SWCLK/SWDIO no shorts, silicon ID confirmed]    │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. FIRMWARE SYNTHESIS & INVARIANT CHECK                                     │
│    - Synthesize minimal, robust firmware (Bare-metal C / Rust #[no_std]).    │
│    - Compile binary & verify vector table invariants via inspect_firmware.  │
│    - Check SP in SRAM bounds, Thumb-bit=1, reset vector valid.              │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 4. DEPLOYMENT & RESET                                                       │
│    programmer_flash ──▶ programmer_reset                                    │
│    [Write chunks, readback verify CRC, assert reset release]                │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 5. PHYSICAL INSTRUMENTATION & PIN CLAIM                                     │
│    pins_claim ──▶ logic_capture_start / logic_capture_trigger               │
│    [Acquire probe channel D0..D7 through Pin Arbiter, arm trigger]          │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 6. WAVEFORM & BEHAVIORAL VERIFICATION                                      │
│    - Measure pulse high/low duration, frequency, jitter, or bus decodes.    │
└───────────────────┬─────────────────────────────────────┬───────────────────┘
                    │ Specs Met (e.g. 1.0Hz ±2%)          │ Signal Missing / Out of Spec
                    ▼                                     ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│ 7a. VERDICT: SUCCESS                 │  │ 7b. AUTONOMOUS SELF-HEALING LOOP  │
│     Output timing proof, edge counts,│  │     target_diagnose (CoreSight)   │
│     and TAP 13 report to user.       │  │     Analyze CFSR/HFSR/Clock stall │
└──────────────────────────────────────┘  │     Patch code, re-flash, re-test │
                                          └───────────────────────────────────┘
```

---

## Safety Invariants & Rules of Engagement

> [!CAUTION]
> **Pin Arbiter Contract**: NEVER sample, drive, or inject signals on AgentLA probe pins without first acquiring ownership through `pins_claim`. The Pin Arbiter protects internal level shifters and avoids bus collisions with active debug or UART sessions. Always call `pins_release` upon task completion.

1. **Explicit Wiring First**: Before building code or touching flash, you MUST tell the user exactly how to wire their hardware. Never proceed to flashing if the user has not confirmed wiring or if electrical rails are absent.
2. **Target Supply Rail Check**: Always check `status` before initiating SWD traffic. If `vtarget_mv < 2700`, do not attempt to flash.
3. **No Phantom Success**: Never declare firmware "working" simply because `programmer_flash` succeeded! Firmware often crashes immediately after reset (clock configuration fault, unaligned pointer, unhandled vector). You MUST measure physical behavior with the logic analyzer or protocol sniffers.

---

## Step-by-Step Execution Protocol

### Step 1: User Wiring & Connection Guidance

When a user specifies a target MCU and desired behavior (e.g., "Write a 1 Hz blink program for STM32F401"):

1. Query the target catalog via `catalog_lookup`:
   ```json
   { "query": "STM32F401" }
   ```
2. Present a clear, table-based wiring guide instructing the user to make the connections:

```markdown
### Required Hardware Connections

Please connect your target MCU to the AgentLA probe as follows:

| Target Signal | Target Pin / Header | AgentLA Probe Pin | Probe Label |
| :--- | :--- | :--- | :--- |
| **VTGT (Sense)** | VDD / 3.3V rail | Pin 28 (VTGT) | `VTGT` |
| **GND** | Target Common GND | Pin 3 or Pin 38 | `GND` |
| **SWCLK** | PA14 (SWCLK) | Pin 24 | `SWCLK` |
| **SWDIO** | PA13 (SWDIO) | Pin 25 | `SWDIO` |
| **RESET** (optional) | NRST | Pin 26 | `NRST` |
| **Verification Pin** | PA5 (User LED) | Pin 0 (D0) | `D0` |
```

3. Confirm electrical viability:
   - Call MCP tool `status`.
   - Ensure `vtarget_mv >= 2700`.
   - Call MCP tool `target_diagnose`.
   - Verify `swclk` and `swdio` are not shorted to GND.

---

### Step 2: Code Synthesis & Vector Table Verification

1. **Synthesize Code**:
   - Write clean, standalone bare-metal C or Rust `#![no_std]` code.
   - For Cortex-M targets, ensure:
     - Target PLL or internal oscillator (HSI) is cleanly started.
     - Peripheral bus clock (RCC AHB/APB) is enabled before writing peripheral registers.
     - GPIO mode register is set to General Purpose Output (push-pull).
     - Delay loop or hardware SysTick timer accurately reflects the CPU clock frequency.
2. **Pre-Flash Vector Table Inspection**:
   - Call MCP tool `inspect_firmware` with `address = flash_base`:
     - Invariant 1: `initial_sp` is word-aligned and points inside SRAM (`ram_base <= initial_sp <= ram_base + ram_size`).
     - Invariant 2: `reset_vector` has bit 0 set to 1 (Thumb-mode flag).
     - Invariant 3: `rdp_level == "Level 0 (unprotected)"`.

---

### Step 3: Flash Programming & Clean Execution

1. Program the flash:
   - Call MCP tool `programmer_flash` with `address` and `hex_data`.
   - Verify `status == "ok"` and written byte count matches payload.
2. Reset target core:
   - Call MCP tool `programmer_reset` with `mode = "sysresetreq"`.
   - Wait $100\text{ ms}$ for power rails and clock PLLs to stabilize.

---

### Step 4: Closed-Loop Physical Signal Verification

To verify that the target MCU is actually executing as intended (e.g., toggling the LED at $1.0\text{ Hz}$):

1. **Acquire Pin Ownership**:
   - Call MCP tool `pins_claim`:
     ```json
     { "mask": 1, "owner": "logic_analyzer", "description": "Verify PA5 blink" }
     ```
2. **Arm Logic Analyzer Capture**:
   - Call MCP tool `logic_capture_trigger`:
     ```json
     {
       "channel_mask": 1,
       "sample_rate_hz": 100000,
       "trigger_channel": 0,
       "trigger_edge": "rising",
       "samples_before": 1000,
       "samples_after": 250000
     }
     ```
3. **Analyze Waveform Metrics**:
   - Calculate measured pulse width ($T_{\text{high}}$), low width ($T_{\text{low}}$), and total period ($T = T_{\text{high}} + T_{\text{low}}$).
   - Frequency $f = \frac{1}{T}$.
   - Check tolerance:
     $$|f_{\text{measured}} - f_{\text{target}}| \le 0.05 \times f_{\text{target}}$$
4. **Release Pin Ownership**:
   - Call MCP tool `pins_release`:
     ```json
     { "mask": 1 }
     ```

---

### Step 5: Autonomous Self-Healing & Debug Loop

If no edges are captured or timing is incorrect:

1. **Trigger Immediate Silicon Triage**:
   - Call MCP tool `target_diagnose`.
2. **Inspect Fault Status Registers**:
   - If `S_LOCKUP == 1`: Target core hit a double fault or watchdog reset.
   - If `CFSR != 0`:
     - `INVSTATE` (bit 17): Missing Thumb bit or invalid branch.
     - `PRECISERR` (bit 9): Attempted to write to GPIO without clocking RCC peripheral bus.
     - `UNALIGNED` (bit 24): Memory access misaligned.
   - If core is in `SLEEP` or halted unexpectedly: Check PLL oscillator startup timeout fallback.
3. **Self-Heal & Iterate**:
   - Adjust the source code to resolve the diagnosed defect (e.g., enable `RCC_AHB1ENR_GPIOAEN`, fix prescaler).
   - Re-flash and re-trigger verification.
   - Repeat autonomously up to 3 iterations before asking the user for hardware intervention.

---

## Output Reporting Standard

Conclude with **TAP 13 (Test Anything Protocol)** and a physical measurement summary table:

```tap
TAP version 13
1..5
ok 1 - User Wiring & Electrical Continuity (VTGT = 3314 mV)
ok 2 - Firmware Vector Table Invariants (SP: 0x20005000, Thumb: 1)
ok 3 - Programming and System Reset (2048 bytes written)
ok 4 - Pin Arbiter Claim (Channel D0 reserved for Logic Analyzer)
ok 5 - Physical Waveform Verification (Target: 1.00 Hz, Measured: 1.002 Hz, Period: 998 ms)
```

| Metric | Target Specification | Measured Physical Value | Tolerance | Verdict |
| :--- | :--- | :--- | :--- | :--- |
| **Supply Voltage** | $3.3\text{ V}$ nominal | $3.314\text{ V}$ | $\ge 2.7\text{ V}$ | **PASS** |
| **Signal Pin** | PA5 (LED) | Channel D0 | Signal present | **PASS** |
| **High Duration** | $500\text{ ms}$ | $499.1\text{ ms}$ | $\pm 5\%$ | **PASS** |
| **Low Duration** | $500\text{ ms}$ | $498.9\text{ ms}$ | $\pm 5\%$ | **PASS** |
| **Frequency** | $1.000\text{ Hz}$ | $1.002\text{ Hz}$ | $\pm 5\%$ | **PASS** |
| **Duty Cycle** | $50.0\%$ | $50.01\%$ | $45\text{--}55\%$ | **PASS** |
