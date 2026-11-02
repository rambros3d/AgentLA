## Why

AgentLA's CH32 programming support works but has significant performance and capability gaps compared to reference implementations. The 1-wire SDI interface uses GPIO bit-banging (~250 KHz) when PIO-based implementations achieve ~2.5 MHz (10x faster). The firmware lacks auto-detection of debug interfaces (SWIO vs SWD), requiring manual mode selection. Flash programming uses simple word-at-a-time writes instead of the PROGBUF injection technique that achieves streaming writes without consuming target RAM. Black Magic Probe cannot flash CH32 MCUs because WCH uses a proprietary RISC-V debug interface incompatible with ARM SWD — AgentLA is one of the few open-source tools that can.

## What Changes

- **SWIO PIO Engine**: Port the PIO-based single-wire protocol from PicoRVD (`singlewire.pio`) to replace GPIO bit-banging in `agentla_ch32.c`. Uses PIO1 SM1 (SM0 reserved for SWD). ~10x speed improvement for CH32V003 operations.
- **Manual MCU Selection**: Replace unreliable auto-detection (ESIG reads returned 0x00000000 → false `chip_id=0x00000000` on real CH32V203 hardware) with explicit MCU selection. `CH32 DETECT mcu=ch32v203` selects the RVSWD two-wire programmer, `mcu=ch32v003` selects SWIO single-wire. The MCU model table supplies canonical chip_id/flash/page parameters, so detection no longer depends on the flaky ESIG read. Legacy `mode=1wire`/`mode=2wire` remains for backward compatibility with test scripts.
- **PROGBUF Flash Injection**: Implement PicoRVD's technique of loading a RISC-V flash write routine into the debug module's program buffer, then using AUTOCMD to stream data to flash. No target RAM required. ~2-3x faster flash writes.
- **CH32V20x/V30x Flash Support**: Add flash programming support for CH32V203/V307 which have different flash register layouts and 256-byte pages (vs 64-byte for V003).
- **Improved Error Handling**: Add retry logic for DMI register access failures, timeout improvements, and diagnostic output for flash operations.

## Capabilities

### New Capabilities

- `ch32-swio-pio`: PIO-based single-wire debug interface for CH32V003 (replaces bit-bang)
- `ch32-manual-select`: Manual MCU model selection (mcu=ch32v203 → RVSWD, mcu=ch32v003 → SWIO)
- `ch32-progbuf-flash`: PROGBUF injection technique for fast flash programming

### Modified Capabilities

_(No existing spec-level behavior changes — all improvements are performance/capability enhancements within existing interfaces.)_

## Impact

- **Code affected**: `firmware/src/programmer/agentla_ch32.c`, `agentla_ch32.h`, `agentla_riscv_dmi.c`, `agentla_riscv_dmi.h`, `agentla_swd.c`. New files: `firmware/pio/agentla_swio.pio`, `firmware/src/programmer/agentla_swio.c`.
- **PIO allocation**: PIO1 SM1 used for SWIO (SM0 reserved for SWD). No conflict with PIO0 (capture engine).
- **APIs**: `cmd_ch32_detect()` gains `mcu=` selection (manual model → transport + flash params). New functions: `ch32_swio_init()`, `ch32_swio_put()`, `ch32_swio_get()`, `ch32_lookup_mcu()`, `ch32_probe_mcu()`. Flash functions gain PROGBUF injection path; V20x/V30x route to `ch32_v20x_flash_*()`. Auto-detection is **removed** — the user selects the MCU model explicitly.
- **Dependencies**: No new external libraries. PIO program is ~30 instructions.
- **Risk**: Medium — PIO timing must match WCH SWIO protocol specs exactly. Test on real CH32V003 hardware required.
