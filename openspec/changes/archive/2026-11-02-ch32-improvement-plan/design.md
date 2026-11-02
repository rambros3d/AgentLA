## Context

AgentLA currently programs CH32 MCUs via two paths:
1. **SWD (ARM standard)** for CH32V203/V307 — uses existing PIO-based SWD engine on PIO1 SM0
2. **1-wire SDI (GPIO bit-bang)** for CH32V003 — slow, CPU-bound, ~250 KHz

Reference implementations (PicoRVD, RVSWD_pico, ESP32-S2 programmer) demonstrate:
- PIO-based SWIO achieves ~2.5 MHz (10x faster than bit-bang)
- PROGBUF injection streams flash writes without target RAM

The goal is to bring AgentLA's CH32 support to reference-quality performance.

## Goals / Non-Goals

**Goals:**
- Replace GPIO bit-bang SWIO with PIO engine (10x speed for CH32V003)
- Manual MCU selection (mcu=) to choose SWIO vs RVSWD transport reliably
- Implement PROGBUF flash injection for faster programming
- Support CH32V20x/V30x flash operations
- Maintain backward compatibility with existing CH32 commands

**Non-Goals:**
- Implementing WCH-LinkE USB protocol (AgentLA uses CMSIS-DAP instead)
- Adding GDB server support (separate feature)
- Supporting CH5xx series (different protocol family)
- Changing the USB interface or host-side protocol

## Decisions

### 1. SWIO PIO Engine — Port from PicoRVD

**Decision**: Port `PicoRVD/src/singlewire.pio` and `PicoRVO/src/PicoSWIO.cpp` to AgentLA.

**Rationale**: PicoRVD's SWIO implementation is the most mature open-source PIO-based single-wire debug engine. It runs at 125 MHz / 12 = ~10.4 MHz PIO clock, matching the WCH SWIO protocol timing requirements (normal mode: 1-bit = low 125-500ns, high 125-2000ns).

**Implementation**:
- New file: `firmware/pio/agentla_swio.pio` — PIO program (same as singlewire.pio with minor adjustments)
- New file: `firmware/src/programmer/agentla_swio.c` — C wrapper providing `swio_put(addr, data)` and `swio_get(addr)` 
- Modify: `agentla_ch32.c` — Replace `ch32_sdi_send_bit/read_bit` with PIO calls
- PIO1 SM1 used (SM0 reserved for SWD engine)

**Alternative considered**: Write custom PIO program. Rejected — PicoRVD's is battle-tested and timing-verified.

### 2. Manual MCU Selection (replaces auto-detect)

**Decision**: Remove auto-detection. The user selects the MCU model explicitly via `CH32 DETECT mcu=<model>`. Each model maps to a debug transport and a set of flash parameters from a static MCU model table.

**Rationale**: Auto-detection proved unreliable on real hardware. On a live CH32V203, the RVSWD link came up (DMSTATUS=0x00000C82) but the ESIG read returned 0x00000000, producing a false-positive `chip_id=0x00000000`. The user always knows which MCU they are programming, so an explicit model parameter is both simpler and more reliable.

**Implementation**:
```
1. Parse mcu= argument → look up in MCU model table
2. Table entry provides: transport (SWIO|RVSWD), chip_id, flash_size, page_size
3. Probe using the selected transport; require verified DMSTATUS link
4. ESIG read is informational only — a 0x00000000 result SHALL NOT fabricate a chip_id
5. Legacy mode=1wire/mode=2wire maps to V003/V20x defaults for test-script compatibility
```

**MCU model table** (initial):
| model | transport | chip_id | flash | page |
|---|---|---|---|---|
| ch32v003 | SWIO | 0x00320500 | 16 KiB | 64 |
| ch32v203 | RVSWD | 0x20305040 | 64 KiB | 256 |
| ch32v307 | RVSWD | 0x30705040 | 256 KiB | 256 |

### 3. PROGBUF Flash Injection

**Decision**: Implement PicoRVD's PROGBUF-based flash write technique for CH32V003.

**Rationale**: PicoRVD's technique streams data through the debug module's program buffer, avoiding the need for target RAM. It uses AUTOCMD (abstractauto) to automatically re-execute the PROGBUF program each time DATA0 is written, achieving continuous streaming writes.

**Implementation**:
- Load 16-halfword RISC-V program into PROGBUF0-7
- Set GPR registers (a0-a5) via abstract commands
- Enable abstractauto=1 for auto-execution
- Stream 32-bit words through DATA0
- PROGBUF handles BUFLOAD, page boundary detection, and page program trigger

**Alternative considered**: Simple word-at-a-time writes (current approach). Rejected — too slow for large binaries.

### 4. CH32V20x/V30x Flash Registers

**Decision**: Add separate flash register definitions for CH32V20x/V30x series.

**Rationale**: CH32V203/V307 use the same RISC-V debug interface (via SWD) but have different flash controller register layouts:
- CH32V003: 64-byte pages, FTPG+BUFLOAD mode
- CH32V20x: 256-byte pages, different CTLR bit assignments
- CH32V30x: 256-byte pages, similar to V20x

Reference: minichlink uses binary blobs for V20x/V30x flash operations. We'll implement the register-level approach.

## Risks / Trade-offs

- **[PIO timing accuracy]** → The SWIO protocol has tight timing requirements. PIO at 125 MHz / 12 gives ~10.4 MHz clock (96ns per tick). This matches normal mode specs. Fast mode may require different clock divider. Mitigation: Test on real hardware, use oscilloscope to verify timing.

- **[PIO1 resource contention]** → SWIO uses PIO1 SM1, SWD uses PIO1 SM0. Both can coexist since they use different state machines. However, only one can be active at a time (they share the same PIO block). Mitigation: Deactivate SWD before activating SWIO, and vice versa.

- **[PROGBUF size limitation]** → PROGBUF is only 8 x 32-bit words (32 bytes). The flash write program must fit in this space. PicoRVD's program is 16 halfwords = 32 bytes, exactly filling PROGBUF. Mitigation: Verified program fits.

- **[CH32V20x/V30x complexity]** → These chips have more complex flash controllers. Minichlink uses binary blobs for reliability. Our register-level approach may need refinement. Mitigation: Start with V003, add V20x/V30x incrementally.

- **[Backward compatibility]** → Existing `CH32 DETECT`, `CH32 ERASE`, `CH32 FLASH` commands must continue working. Mitigation: `mode=1wire`/`mode=2wire` preserved as legacy aliases for `mcu=ch32v003`/`mcu=ch32v203` so existing test scripts keep working.
