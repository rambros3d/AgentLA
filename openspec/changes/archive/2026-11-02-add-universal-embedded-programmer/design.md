## Context

AgentLA is an RP2350 (Pico2) probe with two distinct identities today. As a **debug probe**, it is already general-purpose: CMSIS-DAP v1 over USB HID plus raw SWD over PIO0, so OpenOCD/pyOCD/Keil can debug (and, host-side, flash) any Cortex-M. As a **programmer**, it has a small command surface over CDC (`SWD …`, `FLASH …`, `STM32 TEST`, `DEBUG …`, `CH32 …`) and two on-die flash engines: a hand-written STM32F1 direct-register algorithm in `firmware/src/programmer/target.c`, and an extensive CH32/RISC-V + RVSWD stack. There is also a working host-side ST AN3155 proof of concept (`test/test_an3155_protocol.py`) but it is not wired into firmware.

The core problem: supporting a new MCU in probe-programmer mode means new C code in `target.c` per family, and locked (read-protected) targets cannot be reached at all by debug-direct flashing. With STM32F4/G0/H7, Nordic and WCH in scope, this does not scale.

## Goals / Non-Goals

**Goals:**
- Make probe-driven programming work for any MCU that supplies (or can be reached by) an official flash algorithm and/or a vendor ROM bootloader.
- Recover RDP Level 1 (mass-erase-permitted) targets through SWD debug.
- Provide explicit, safe command flows with identity verification and confirmation.
- Keep CMSIS-DAP debug, and RVSWD/SWIO (CH32), untouched.

**Non-Goals:**
- RDP Level 2 / permanently locked parts — unrecoverable by definition; out of scope.
- Foolproof flashing of parts with no ROM bootloader and no accessible DP — not supported.
- RP2040/2350 UF2 mass-storage mode — orthogonal; not in this change.
- CMSIS-DAP v2 speed upgrade — separate, later change.
- Full firmware debugger / GDB server on the probe.

## Decisions

### D1: Multi-method, not one silver bullet
One mechanism cannot cover "all vendors + locked recovery" (mass-erase needs an unlocked DP; bootloader needs BOOT0/nRST). Instead a policy layer (method selector) picks:
- unlocked + family known → Direct register path (fastest, still used as a benchmark and during transition);
- unlocked + any family → Generic algorithm runner (single code path, everything with an official algorithm);
- locked, DP reachable → Debug mass-erase, then reflash;
- debug not usable / RDP Level 2 unreachable via mass-erase → Bootloader recovery.
This is the only shape that simultaneously covers normal and locked cases. A single method would silently fail on one or the other.

### D2: Generic algorithm runner becomes the primary engine, but direct is retained until parity
Rationale: Keil/CMSIS Packs already ship and maintain per-device algorithms; duplicating their logic in C only creates drift. The probe will: halting the core, downloading a blob into SRAM, setting up registers, calling fixed ABI entry points via the CoreSight debug interface (DCRSR/DCRDR), and reading results. The runner must first match or beat direct-algorithm performance and reproduce the existing STM32F1 5/5 suite (`test_stm32f1_hardware_suite.py`) before direct `stm32f1_*` code is removed; until then both coexist. Alternatives considered: keeping both forever (rejected — double maintenance) or hand-coding F4/G0/H7 only (rejected — doesn't address cross-vendor).

### D3: Bootloader recovery uses a dedicated BOOT0 pin
BOOT0 is allocated as its own GPIO (default **GPIO2**, push-pull drive) separate from nRST (GPIO6) and power enable, so recovery does not overload a board's power-enable line. GPIO2 is free in programmer mode, clear of SWD (4/5), nRST (6), UART (8/9) and I2C (0/1); GPIO3 remains available as a spare if needed. The pin is selected in `board_settings.h`, but the compiled-in default is GPIO2. The signal is driven push-pull for reliable BOOT0 levels on STM32 targets. The exact pin and wiring are a board-level change, documented alongside nRST. Alternative considered: reusing GPIO7 as BOOT0 (rejected — conflicting with power-enable on some boards) and external jumper only (rejected — beats the standalone goal).

### D4: Host-side `.flm → blob` conversion, not on-probe ELF parsing
The probe only runs flat ARM Thumb blobs with a small descriptor (entry offsets, SRAM window, stack). Parsing/validating ELF or vendor packs on an RP2350 is out of scope and risky. A Python converter in `sdk/` (or `tools/`) produces `.blob` + JSON metadata that the probe consumes. This reuses the strong Python side of the project.

### D5: Vendor set for v1 = STM32 + Nordic + WCH
These are the families already exercised by `target.c` and `rvswd.c`/`swio.c`. ESP32/NXP remain future method modules (additional boot protocols). Sequencing keeps v1 bounded.

### D6: Safety rails as first-class requirements
Destructive operations require (a) successful target identification and size read, (b) detected protection level shown to the user, and (c) explicit confirmation via the existing `CONFIRM`-style token or a new `--confirm` argument, plus a "you are about to erase <N> bytes on <part>" banner.

## Risks / Trade-offs

- [Algo-runner throughput unknown] Running an algorithm across SWD could be slower than direct; mitigation: benchmark vs. current F1 suite before deleting direct code, allow keeping a direct path per-part in transitional builds.
- [Wrong-part brick risk] Mis-detection could erase the wrong flash; mitigation: mandatory identity + geometry check before any erase; reject when protection level is unknown.
- [Target code execution is trust-critical] Uploaded blobs run with the target's privileges; mitigation: explicit consent, documented source requirement (official packs only), optional compile-time disable.
- [BOOT0 is a board-level wiring change] Bootloader recovery needs a dedicated BOOT0 GPIO rather than GPIO7; mitigation: select the pin via `board_settings.h`, document the wiring, and ship a software-only mode where recovery is unavailable if unwired.
- [RDP detection is optimistic] The probe attempts mass-erase instead of reliably classifying Level 1 vs Level 2 first; mitigation: report the attempt outcome explicitly and require post-erase re-identification before programming.
- [No on-probe blob storage (v1)] Algorithm blobs are streamed per session over CDC/USB; mitigation: keep the stream format compact and treat persistent caching as a later enhancement.
- [Locked/blocked bootloader cases remain open] Recovery of parts with permanently removed ROM bootloaders needs more research; mitigation: keep it in the decision log and spike notes, not in the v1 implementation.
- [Collisions with in-flight changes] `import-rvswd-pio-transport` and `vendor-picorvd-progbuf-mem-access` touch overlapping programmer code; mitigation: keep this change dependent on those completing their touched files; merge only after.

## Migration Plan

1. Hardware spikes (Spike A: algo-runner primitive reproducing F1 suite; Spike B: dedicated-pin BOOT0 + AN3155 recovery; Spike C: optimistic mass-erase on a locked part).
2. Implement algo-runner core plus optional direct algorithms side-by-side; validate.
3. Add mass-erase recovery (ARM) and bootloader modules (ST/WCH); hardware-gate each.
4. Convert F1 to use algo-runner as primary; delete F1 direct code; update specs.
5. Add Nordic mass-erase path; document safety rails.
6. Rollback strategy: keep direct `stm32f1_*` available behind a feature flag until F1 parity + recovery pass on live hardware.

## Findings (Spike results)

- **Spike A (algo exec primitive): PASS on hardware** — `ALGOTEST` returns `0x00000042` from a Thumb function executed in SRAM via the runner's halt/download/seed/resume/BKPT/read path. Core register round-trip is therefore viable as the basis of the generic flash algorithm runner.
- **Spike B (BOOT0 control): PASS** — `BOOT0 HIGH/LOW` command controls GPIO2 without build errors or regressions; wiring of GPIO2→BOOT0 is functional and non-destructive to the SWD path. End-to-end AN3155 recovery remains blocked on UART pin remapping (adapter UART vs programmer UART).
- **Level-2 / permanently locked research**: Setting RDP Level 2 (or permanently disabling ROM bootloaders via option bytes) is irreversible on STM32 — no SWD, ST-Link, CubeProgrammer, AgentLA, or UART path can recover. The only recoveries are Level-1 mass erase (recoverable via ST-Link, OpenOCD, or `MASSERASE CONFIRM` in AgentLA/agent target), connect-under-reset (for non-protection faults), or keeping the target in a trusted state. **v1 scope deliberately excludes any reversible unlock of Level-2.**
- **Baseline regression:** SWD detection, F1 hardware suite (5/5), and mass-erase all passed on 2026-10-28 hardware.
- **Open limit:** the generic runner still lacks the Init/Erase/Program/Verify/UnInit orchestration and trust/consent UX; real algorithm injection remains a follow-up.

- **Q1 (performance parity):** Generic execution must match or beat direct-algorithm speed on F1 *before* direct code is removed. Status: gated by Spike A + task 3.6.
- **Q2 (BOOT0 pin):** A dedicated BOOT0 GPIO will be allocated (not GPIO7); exact pin and board wiring are a design task (task 5.1 / hardware note).
- **Q3 (RDP level):** Optimistic-try approach is selected — attempt mass-erase and report the result; no upfront Level 1/2 classification required.
- **Q4 (blob storage):** Stream-only for v1; no on-probe persistence.
- **Q5 (permanently locked / ROM-less parts):** Under further research; out of the v1 release scope but kept on the spike/decision log.
- New open question: Which physical GPIO becomes the dedicated BOOT0 pin without conflicting with logic analyzer/adapter modes?