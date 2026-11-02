## Context

AgentLA probes (Pico 2 and BakuProbe) operate as versatile multi-instrument workstations across embedded protocols, logic capture, analog oscilloscopes, and device flashing. Currently, all instrument configurations and pin routings must be re-applied manually upon connecting. Furthermore, live hardware setups require an absolute guarantee that booting or switching projects will not accidentally drive unconfigured voltages into target chips.

This design introduces a hardware-resident profile architecture with 32 on-device flash slots and an emergency Safe State system.

## Goals / Non-Goals

**Goals:**
- Store up to 32 complete workbench profiles directly in the RP2350 non-volatile flash memory (64 KB dedicated block).
- Guarantee that every power-on, reset, or hot-unplug defaults to 100% High-Z safe inputs and 0.0V target power.
- Support an optional, user-designated boot default profile that auto-applies on power-up.
- Provide an instantaneous emergency Safe State trigger via a prominent WebUI header button and `Shift+Esc` hotkey.
- Implement graceful cross-board partial loading when loading profiles across different hardware targets (e.g. BakuProbe to Pico 2).
- Ensure safe electrical transitions (quiescence, 5ms intermediate tri-state, power discharge) during profile switches.

**Non-Goals:**
- Cloud-synced user account storage (profiles reside on the physical probe hardware).
- Dynamic runtime overclocking adjustments per profile.

## Decisions

### 1. Flash Memory Map: 32 Slots in 64 KB Block
- **Decision**: Reserve the upper 64 KB of flash (`PICO_FLASH_SIZE_BYTES - 64KB`). Each profile slot is assigned 2048 bytes (2 KB).
  - Slot 0: Factory-defined permanent `Safe State (All Hi-Z, 0.0V)` (Read-Only).
  - Slots 1–31: User-configurable persistent profile slots.
- **Rationale**: 64 KB is negligible on 4MB (Pico 2) and 16MB (BakuProbe) chips (<1.5% of flash). 2 KB per slot provides ample room for minified JSON describing 48 pins, 16 logic analyzer channels, 8 scope channels, bus baud rates, and decoders.
- **Header Structure**:
  ```c
  typedef struct __attribute__((packed)) {
      uint32_t magic;         // 0x41474C41 ('AGLA')
      uint16_t version;       // 1
      uint16_t flags;         // bit 0: in_use, bit 1: is_boot_default
      char name[32];          // Human-readable profile name
      uint32_t payload_crc32; // Checksum of JSON payload
      uint32_t payload_len;   // Byte length of JSON payload
      uint8_t payload[1996];  // Minified JSON workbench state
  } profile_slot_t;
  ```

### 2. Default Safe State with Optional Boot Override
- **Decision**: Firmware boots with all GPIOs set to High-Z inputs with pulls disabled, VTARGET MOSFET off, and transceivers unpowered. The profile manager scans flash slots for `is_boot_default`. If found, it executes a safe delayed transition (50ms) to apply that profile; otherwise, it remains in Safe State awaiting WebUI commands.
- **Rationale**: Maximizes hardware protection while accommodating dedicated production or automated test benches where permanent target connections exist.

### 3. Atomic Pin Handover Engine
- **Decision**: In `firmware/src/profile_manager.c`, implement `profile_apply_safe(slot_id)`:
  1. Abort active DMA channels and PIO state machines.
  2. Call `pin_owner_quiesce_and_reset()` to revoke all claims and set all pins to High-Z SIO inputs.
  3. Turn off VTARGET power rail and wait 5ms to discharge target capacitance.
  4. Parse the target profile's pin map, validating against current board constraints via `pin_owner.c`.
  5. Apply new pin claims atomically (reverting to Safe State if a claim conflict occurs).
  6. Re-enable VTARGET with current limiting if requested by the profile.
  7. Re-initialize instrument peripherals (Logic Analyzer, Oscilloscope, Bus Adapters).

### 4. Cross-Board Graceful Partial Loading
- **Decision**: When the WebUI client loads a profile whose `board_id` differs from the connected board:
  - Validates each pin assignment against the current board's GPIO count and zone constraints.
  - Applies compatible configurations (e.g. GP0-GP25 on Pico 2).
  - Skips hardware absent on the current board (e.g. TCAN334 CAN-FD or GP40-GP47 on Pico 2).
  - Displays a modal summarizing: *"Applied X compatible pins/instruments; skipped Y features not present on this hardware"*.
- **Rationale**: Prevents cryptic failure errors when engineers share test setups across team members using different board revisions.

### 5. WebUI Header Integration & Emergency Controls
- **Decision**: Add `[ 🛡️ SAFE STATE ]` button to the primary WebUI header bar alongside mode navigation, bound to the `Shift+Esc` hotkey. Add a top-bar profile selector dropdown displaying the active profile with a color-coded status badge.

## Risks / Trade-offs

- **[Risk] Flash Wear from Frequent Profile Writes** → Flash endurance on RP2350 is ~100,000 erase cycles per sector.  
  *Mitigation*: Profile slots are saved only on explicit user action ("Save as Profile"), not on every parameter tweak in the UI. 100,000 writes exceeds decades of normal bench usage.
- **[Risk] Incomplete Write on Sudden Power Cut** → Operator unplugs probe while writing a 2KB profile slot.  
  *Mitigation*: Checksum validation (CRC32) in slot header. Corrupted slots fail CRC check on boot and are marked as uninitialized.
