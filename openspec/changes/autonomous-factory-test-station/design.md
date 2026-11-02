## Context

Modern electronics manufacturing lines require rapid, repeatable, and automated programming and functional testing of populated printed circuit boards (PCBs). Bed-of-nails test fixtures utilize spring-loaded pogo pins to access test pads on the un-cased PCB.

AgentLA currently provides manual, interactive target debugging, flashing, and pin inspection. This design provides the technical architecture for an **Autonomous Factory Test Station** (targeted for post-v1 releases) that turns AgentLA / BakuProbe into an unattended production test instrument.

## Goals / Non-Goals

**Goals:**
- Provide sub-4-second end-to-end flash programming, serialization, and functional circuit verification (FCT).
- Provide hands-off automatic triggering upon mechanical fixture closure / pogo pin contact.
- Provide hardware-level short-circuit protection using BakuProbe's TI INA180 current sense circuit (<1ms power disconnect).
- Support declarative JSON test recipes defining target parameters, binaries, and peripheral test steps.
- Offer dual-mode operation: a full-screen, high-contrast WebUI surface with audio feedback for operators, and a headless Python runner (`tools/production_runner.py`) for factory MES databases.

**Non-Goals:**
- In-circuit boundary scan (JTAG IEEE 1149.1 BS-DL) or automated optical inspection (AOI).
- Replacement of high-voltage dielectric isolation testing (hipot).
- Implementation in the v1.0 initial release (this proposal captures the future architecture for post-v1 planning).

## Decisions

### 1. Autonomous Contact Sensing & State Machine
- **Decision**: Implement a low-latency autonomous state machine in firmware:
  ```
  [IDLE] ──(VTARGET > 1.2V for 150ms)──► [PRE_FLIGHT_SAFETY]
                                               │
               ┌───────────────────────────────┴───────────────────────────────┐
               ▼ (Current > limit: TRIP)                                       ▼ (Current OK)
         [FAULT_SHORT]                                                  [SWD_IDENTIFY]
                                                                               │
                                                                               ▼
                                                                        [FLASH_PROGRAM]
                                                                               │
                                                                               ▼
                                                                        [FCT_PERIPHERAL]
                                                                               │
                                                                               ▼
                                                                        [PASS_EJECT]
  ```
- **Rationale**: Relying on operator clicks adds human latency and error. Autonomous sensing allows operators to simply clamp down, wait for the green light, and unclamp.

### 2. Fast Hardware Pre-Flight Overcurrent Trip
- **Decision**: Sample the INA180 current monitor (GP46 / ADC6) before releasing target reset. If quiescent current exceeds `max_short_current_ma` (e.g. > 100mA), an interrupt or fast ADC threshold monitor instantly trips the target power MOSFET in under 1 millisecond.
- **Rationale**: Prevents charred PCB traces or damaged pogo pins when defective boards with solder bridges are clamped down.

### 3. Declarative Recipe Engine (`factory_recipe.json`)
- **Decision**: Define test flows via declarative JSON rather than bespoke test scripts.
- **Rationale**: Allows production line managers and test engineers to update firmware versions, test limits, and serialization rules without modifying firmware C code or core WebUI logic.

### 4. Dynamic Serialization Injection
- **Decision**: The test runner dynamically patches the serial number, MAC address, and calibration data directly into the pre-allocated flash sector before writing the binary or via a post-flash RAM stub.
- **Rationale**: Eliminates the need to pre-generate hundreds of unique binary files before a production run.

### 5. High-Contrast WebUI Surface & Headless Runner
- **Decision**: Build a specialized Production Station UI view with massive green/red full-screen indicators, audio chimes, and live cycle time statistics, alongside a Python CLI tool communicating over USB RNDIS HTTP / JSON-RPC.
- **Rationale**: Shop-floor operators need immediate, glanceable feedback from meters away. Automated factory lines need headless CLI integration into factory MES servers.

## Risks / Trade-offs

- **[Risk] Pogo Pin Contact Bounce** → Mechanical vibration when clamping can cause micro-disconnects.  
  *Mitigation*: Implement a 150ms stable-voltage debounce window before asserting SWD communication.
- **[Risk] High-Speed In-Circuit Programming Failures** → Long pogo pin leads can introduce capacitive loading and signal ringing on SWD clock/data lines.  
  *Mitigation*: Provide configurable SWD clock speeds in recipe (e.g. 1 MHz fallback up to 10 MHz nominal) and slew-rate control.
- **[Risk] Incomplete Flash on Premature Unclamp** → Operator opening clamp mid-write could brick the board.  
  *Mitigation*: The system immediately logs a FAIL record with an invalid CRC flag in the database upon sudden VTARGET drop.
