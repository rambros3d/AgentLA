## Why

The Rust port only scaffolds state; real bus driving (I2C/MCP2221, SLCAN, WS2812 via neopixel.pio, MIPI-I3C) is coded as state toggles but never wired. The product goal requires a **host-usable universal bus adapter** in addition to the web/programmer, and Phase 5 (MCP desktop) depends on Phase 4's real ability.

## What Changes

- **I2C tiny-USB emulation**: replace in-RAM state with open-drain I2C adapter through `pin_owner` flex pins (16/17), standard ACK and reset handling, 100k mode, R/W transactions framed exactly as MCP2221 (`0x10`, `0x20`, `0x30`).
- **SLCAN**: convert ASCII `S/O/C/t/T/r/R` router to a real bus controller that takes timing from drive loop, emits host-friendly can0 state.
- **WS2812**: actual PIO (`neopixel.pio`, side-set PINDIR as open-drain; 800 kHz divider) driving a small ring (4-8 px).
- **MIPI-I3C**: SDR transaction (START/ADDRESS/STOP), common CCCs (`ENTDAA`, `RSTDAA`) and dynamic-address pushback.

## Capabilities

### New
- `bridge-i2c-tiny-mcp-real`
- `bridge-slcan-real`
- `bridge-led-ws2812-real`
- `bridge-i3c-real`

### Modified
- `pin-ownership` — flex pins reserved at claim-time; clear safety on release.
- `rest-hardware-profiles` — alias requests unchanged.

## Impact
- `firmware-rs/src/bridge/{i2c,slcan,ws2812,i3c}.rs` and REST hooks build (no new deps)
- Testing via `curl` on `/api/v1/bridge/*`; safe state guaranteed on claim failure.
