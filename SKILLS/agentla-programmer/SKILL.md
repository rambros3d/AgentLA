---
name: agentla-programmer
description: Universal target flash programmer and unbrick specialist for ARM Cortex-M, RISC-V, and RP2040/RP2350 microcontrollers. Manages memory map catalogs, sector-aligned flashing, CRC verification, and silicon unlock sequences.
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  target_architectures: ["STM32", "RP2040", "RP2350", "nRF51/52", "CH32V", "SAMD", "ESP32-C3/C6"]
  tools_required: ["agentla-mcp"]
---

# Target Flash & Unbrick Specialist (`agentla-programmer`)

This skill equips an AI agent to safely flash, verify, and rescue microcontrollers across multiple silicon vendors using AgentLA's hardware programming subsystem and on-probe chip catalog.

---

## Programming & Unbrick Execution Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. TARGET CATALOG & MEMORY BOUNDS BINDING                                   │
│    catalog_lookup ──▶ probe_target                                          │
│    [Retrieve: Flash Base, Page/Sector layout, RAM staging buffer, Algorithms]│
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. SILICON PROTECTION AUDIT (RDP / APPROTECT)                               │
│    inspect_firmware ──▶ programmer_unbrick (if locked)                      │
│    [Check: STM32 RDP Level 0, nRF APPROTECT disabled, RP2350 OTP valid]     │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. CHUNKED FLASH PROGRAMMING & CRC VERIFICATION                             │
│    programmer_flash ──▶ programmer_read                                     │
│    [Write chunked buffers, stream CRC32 / SHA256 readback comparison]       │
└───────────────────┬─────────────────────────────────────┬───────────────────┘
                    │ Verification Pass                   │ Write Fail / Corrupt
                    ▼                                     ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│ 4a. SYSTEM RESET & LAUNCH            │  │ 4b. MULTI-VENDOR RESCUE ENGINE    │
│     programmer_reset (SYSRESETREQ)   │  │     CTRL-AP mass erase / unbrick  │
│     Target booted into new firmware. │  │     Restore factory Option Bytes  │
└──────────────────────────────────────┘  └───────────────────────────────────┘
```

---

## Safety Invariants & Flash Rules

> [!CAUTION]
> **RDP Level 2 Warning**: STM32 chips locked with **RDP Level 2** (permanent hardware lock) have JTAG/SWD disabled in silicon mask. Under NO circumstances can Level 2 be unlocked; attempting to brute-force debug commands will permanently fail. RDP Level 1, however, CAN be rescued via mass erase.

1. **Target Rail Voltage Gate**: NEVER call `programmer_flash` if `vtarget_mv < 2700`. Flash charge pumps (which generate $12\text{--}18\text{ V}$ internally for floating gate tunneling) fail below $2.7\text{ V}$, causing permanent flash ECC errors or partial writes.
2. **Sector Boundary Alignment**: Ensure write operations respect the target chip's minimum programming granularity (e.g., 32-bit words for STM32F1/F4, 64-bit for STM32G4/L4, 256-byte pages for SPI NOR on RP2040/RP2350).
3. **Mandatory Post-Write Readback**: Never report flashing as complete without verifying at least the first 256 bytes (vector table) and a CRC32 check of the written payload.

---

## Step-by-Step Programming Protocol

### Phase 1: Chip Catalog Lookup & Memory Layout

1. Query target catalog using MCP tool `catalog_lookup`:
   ```json
   { "query": "STM32F401RE" }
   ```
2. Parse catalog parameters:
   - `flash_base`: Start address of internal Flash (e.g., `0x08000000`).
   - `flash_size`: Total flash size (e.g., `0x00080000` = 512 KB).
   - `sector_sizes`: Sector division (e.g., 4 x 16 KB, 1 x 64 KB, 3 x 128 KB).
   - `ram_base`: Start of internal SRAM (e.g., `0x20000000`).
   - `ram_size`: Total RAM size (e.g., `0x00018000` = 96 KB).
3. Verify target presence with `probe_target`:
   - Match reported `family` with catalog data.

---

### Phase 2: Protection State Audit & Unbricking

Before programming, check if security locks prevent flash access:

1. Call MCP tool `inspect_firmware`:
   - Read `rdp_level` and `security_bits`.
2. **Vendor-Specific Rescue Playbooks**:
   - **STM32 RDP Level 1**:
     - Symptoms: Core connects via SWD, but reading flash addresses (`0x08000000`) returns all zeroes or generates BusFault.
     - Solution: Call `programmer_unbrick`:
       ```json
       { "family": "stm32", "action": "rdp_unlock" }
       ```
       *Note: This triggers a hardware mass erase, clearing all flash and resetting RDP to Level 0.*
   - **Nordic nRF52 APPROTECT**:
     - Symptoms: Debugger cannot attach to MEM-AP; only CTRL-AP (`AP[1]`) is accessible.
     - Solution: Call `programmer_unbrick`:
       ```json
       { "family": "nrf52", "action": "erase_all" }
       ```
   - **RP2040 / RP2350 Flash Rescue**:
     - Symptoms: Misconfigured QSPI flash clock or watchdog loop prevents debugger attach.
     - Solution: Call `programmer_unbrick`:
       ```json
       { "family": "rp2040", "action": "rescue_reset" }
       ```

---

### Phase 3: Chunked Flash Programming & CRC Verification

1. **Erase Target Sectors / Mass Erase**:
   - For complete image overwrite, invoke mass erase or sector erase covering the binary size.
2. **Stream Binary in Chunks**:
   - Call MCP tool `programmer_flash`:
     ```json
     {
       "address": 134217728,
       "hex_data": "00500020810100088501000887010008..."
     }
     ```
   - Stream successive chunks (1024 or 2048 bytes per call) until entire image is written.
3. **Verify Integrity**:
   - Call MCP tool `programmer_read` to sample the vector table and trailing chunks:
     ```json
     {
       "address": 134217728,
       "length": 64
     }
     ```
   - Verify that bytes match the original binary payload.

---

### Phase 4: Launch & Verification

1. Reset target core:
   - Call MCP tool `programmer_reset` with `mode = "sysresetreq"`.
2. Verify core boots out of reset:
   - Call `target_diagnose` after $100\text{ ms}$.
   - Ensure `DHCSR.S_HALT == 0` (core running) and `DHCSR.S_LOCKUP == 0` (no fault).

---

## Output Reporting Standard

```tap
TAP version 13
1..4
ok 1 - Target Catalog Lookup (STM32F401RE: 512KB Flash @ 0x08000000)
ok 2 - Protection State Audit (RDP Level 0 Unprotected)
ok 3 - Flash Programming (14,336 bytes written across 7 chunks)
ok 4 - Integrity Readback Verification (CRC32 match: 0x9A4E21D0)
```

| Parameter | Specification | Flashed / Verified Value | Status |
| :--- | :--- | :--- | :--- |
| **Target MCU** | STM32F401RET6 | ARM Cortex-M4 (0x2BA01477) | **MATCH** |
| **Base Address** | `0x08000000` | `0x08000000` | **VALID** |
| **Payload Size** | 14,336 Bytes | 14,336 Bytes | **WRITTEN** |
| **Write Time** | $< 1.5\text{ s}$ | $680\text{ ms}$ | **PASS** |
| **Verification** | Byte-by-byte SHA256 | CRC / Readback Confirmed | **PASS** |
