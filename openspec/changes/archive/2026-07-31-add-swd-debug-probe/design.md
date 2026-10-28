## Context

The AgentLA firmware currently supports:
- **UART bootloader flashing** (GPIO 4-5) — Vendor-specific protocols for STM32, RP2350
- **Logic analysis** (GPIO 20-29) — PIO0-based capture engine
- **Adapter interfaces** (GPIO 10-19) — I2C, UART, SPI, CAN

**Current state:** SWD pins (GPIO 6-7) are already allocated on PIO1 but unused. The debugprobe reference implementation (MIT licensed) provides a proven PIO-based SWD engine.

**Constraints:**
- PIO1 handles SWD without conflicting with PIO0 (capture)
- USB VID/PID 0x1209/0x414C reserved for analyzer mode
- Must maintain backward compatibility with existing UART bootloader

## Goals / Non-Goals

**Goals:**
- Implement CMSIS-DAP v2 compatible SWD debug probe
- Support all STM32 families (F1, F4, G0) and RP2040/RP2350
- Auto-detect target MCU via IDCODE
- Work with OpenOCD, pyOCD, and GDB
- Configurable SWD speed (1-4 MHz)

**Non-Goals:**
- JTAG support (future, GPIO 8-9 reserved)
- SWO capture (optional, can add later)
- CMSIS-DAP v1 compatibility (v2 is sufficient)

## Decisions

### 1. PIO-based SWD Engine (from debugprobe)

**Decision:** Port debugprobe's `probe.pio` to PIO1 on RP2350.

**Rationale:**
- PIO handles tight SWD timing without CPU intervention
- 1-cycle turnaround (TRN) trivial in PIO
- Bit-bang up to 256 bits per command
- Proven in production (debugprobe)

**Alternatives considered:**
- Bit-bang in C: Too slow for 4 MHz SWD, CPU-bound
- SPI peripheral: Not bidirectional, can't handle turnaround
- UART peripheral: Not designed for SWD protocol

**PIO Program Structure:**
```
┌─────────────────────────────────────────────────────────────┐
│  PIO State Machine                                          │
│                                                             │
│  TX FIFO ──▶ Command Parser ──▶ Bit Loop ──▶ RX FIFO       │
│                                                             │
│  Commands:                                                  │
│  - write_cmd: Drive SWDIO, generate SWCLK                  │
│  - read_cmd: Sample SWDIO on SWCLK rising edge             │
│  - turnaround_cmd: Release SWDIO for 1 cycle               │
└─────────────────────────────────────────────────────────────┘
```

### 2. USB HID Interface with Different PID

**Decision:** Use VID:0x1209 PID:0x414D for SWD mode.

**Rationale:**
- Different PID allows host tools to auto-detect function
- Same VID maintains vendor identity
- Avoids confusion between analyzer and debug modes

**Alternatives considered:**
- Same PID with function descriptors: More complex USB stack
- Composite device: Overkill for single function

### 3. Full CMSIS-DAP Command Set

**Decision:** Implement complete CMSIS-DAP v2 command set.

**Rationale:**
- Maximum compatibility with host tools
- Future-proof for additional features
- debugprobe reference provides working implementation

**Core commands:**
- DAP_Info, DAP_Connect, DAP_Disconnect
- DAP_Transfer (SWD read/write)
- DAP_SWJ_Clock (speed control)
- DAP_SWJ_Sequence (reset, JTAG2SWD)
- DAP_ResetTarget, DAP_TransferConfigure

### 4. Flash Algorithm Interface

**Decision:** Create pluggable flash algorithm architecture.

**Rationale:**
- Different MCU families have different flash controllers
- Auto-detect requires runtime algorithm selection
- Easy to add new targets without changing core code

**Interface:**
```c
typedef struct {
    const char *name;
    uint32_t idcode;
    bool (*flash_erase)(uint32_t sector);
    bool (*flash_write)(uint32_t addr, uint8_t *data, uint32_t len);
    bool (*flash_read)(uint32_t addr, uint8_t *data, uint32_t len);
} flash_algorithm_t;
```

### 5. UART Bootloader as Fallback

**Decision:** Keep existing UART bootloader code, make it optional.

**Rationale:**
- Recovery option if SWD fails
- Useful for chips without SWD
- Already implemented and tested

**Trade-off:** Slightly larger firmware, but acceptable for reliability.

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| PIO timing issues at 4 MHz | Start with 1 MHz, test incrementally |
| USB HID descriptor complexity | Reference debugprobe implementation |
| Flash algorithm bugs per target | Test each MCU family thoroughly |
| Pin conflicts with UART mode | Mode detection via SWD INIT success |
| CMSIS-DAP protocol edge cases | Use proven debugprobe protocol layer |

## Migration Plan

1. **Phase 1:** PIO SWD engine (standalone, testable)
2. **Phase 2:** CMSIS-DAP protocol layer
3. **Phase 3:** USB HID interface
4. **Phase 4:** Flash algorithms
5. **Phase 5:** Integration testing

**Rollback:** Each phase is independently testable. If issues arise, revert to previous phase.

## Open Questions

- [ ] Should we support SWO capture in v1 or defer to v2?
- [ ] How to handle flash protection (RDP Level 2)?
- [ ] Should vendor-specific commands be documented in spec?
