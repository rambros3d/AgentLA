## Context

AgentLA supports 2-wire RV-SWD debugging and programming for RISC-V CH32V203 / CH32V307 MCUs on GPIO4 (`SWCLK`) and GPIO5 (`SWDIO`). To perform end-to-end hardware verification on physical CH32V203 targets, we need an automated multi-stage test suite that validates link setup, register read/write, fast page flash programming, and binary verification.

## Goals / Non-Goals

**Goals:**
- Design a structured automated hardware verification test script `test/test_ch32v203_hardware_verification.py`.
- Verify serial CDC CLI communication (`IDENTIFY`).
- Verify target detection and electronic signature (ESIG at `0x1FFFF704`) over 2-wire RV-SWD (`CH32 DETECT mode=2wire`).
- Verify sector erase (`CH32 ERASE`) and fast buffer loading page write (`CH32 FLASH`).
- Verify exact binary readback and payload comparison (`FLASH READ`).

**Non-Goals:**
- Debugging 1-wire SDI on CH32V003 in this specific test suite (handled in `test_ch32v003.py`).

## Decisions

### 1. Multi-Pass Verification Pipeline
- **Choice**: Execute a sequential 5-stage test pipeline: 1. Serial Handshake, 2. Target Probe, 3. Sector Erase, 4. Fast Page Write, 5. Readback Comparison.
- **Rationale**: Isolates failures immediately to the exact layer (serial -> SWD physical link -> DMI registers -> Flash controller -> Flash memory cells).

### 2. Hex Payload Verification with Random/Blink Pattern
- **Choice**: Program a 128-byte test pattern payload and read back memory via `FLASH READ` to perform byte-for-byte exact equality checks.
- **Rationale**: Ensures both flash write and memory bus read paths function properly without bit corruption.

## Risks / Trade-offs

- **[Risk] Serial Port Contention / Zombie Processes** → **Mitigation**: Add automatic process lock cleanup (`fuser`) and explicit DTR/RTS assertions in python script initialization.
- **[Risk] Target Power Loss during Erase** → **Mitigation**: Add timeout bounds and status check retries before failing test stages.
