## Why

AgentLA already has a mature set of bus bridges (I2C tiny-USB, SLCAN, MIPI I3C master, WS2812 LED), MIPI-I3C decoder in the C firmware but the Rust port has none — host-side USB bus sniff packages that target the tool today can't work without them. Pin arbitration + PWM (Phase 2) and LittleFS (Phase 1) are complete, removing the last blocker (shared-pin conflicts) and giving ready state.

## What Changes

- **I2C tiny-USB / MCP2221 emulation bridge**: USB high-speed VCOM → I2C via the existing Phase2 pin owner claim (open-drain SIO pins as I2C alternates, `adapter_i2c_usb.c` parity) — passthrough reads/writes with timeout.
- **SLCAN ASCII protocol (SLCAN)**: ACM serial `SA..SN` commands mapped to USB CDC-ACM or virtual `can0`; uses `adapter_can.c` property semantics.
- **I3C master adapter** (MIPI I3C): SDR/HDR-CCC commands using the `i3c.pio` program lifted from `adapter_i3c.c`; safe dynamic re-arm.
- **NeoPixel/WS2812 driver** using `neopixel.pio` lift from `adapter_led.c`.
- **Dynamic UART bridge**: extend existing fixed CDC-ACM UART to multi-baud dynamic reconfigure (existing Pin Owner claims).
- **CLI routing** add `BRIDGE` and `ADAPTER-*` commands in `web`/`programmer` surfaces.

## Capabilities

### New Capabilities
- `bridge-i2c-tiny-mcp`: MCP2221 emulation protocol contract and pin claim.
- `bridge-slc-can`: SLCAN ASCII host-side contract for CAN passthrough.
- `bridge-i3c-master`: I3C SDR master transactions.
- `bridge-uart-dynamic`: multi-baud as needed.
- `bridge-led-ws2812`: simple-neopixel driver against pio/neopixel.pio.

### Modified Capabilities
- `pin-ownership` — bridges can claim `I2C`, `UART`, `PIO` flexible pins only and only when unclaimed.

## Impact

- New modules live in `firmware-rs/src/bridge/*.rs` (i2c_usb, can_slcan, i3c, uart, ws2812, core adapter), plus `ctl` surface endpoints.
- PIO program additions (`i3c.pio`, `neopixel.pio`, `urx`/`utx`) in `pio_programs` module alongside existing pins.
- USB CDC-ACM endpoints already exist (already in `usb.rs`) reused.
- hardware pin usage fixed: `i2c` pair from 16/18? based on flexible registry claim outside 4-14; (26-29 ADC). Rate-driven limits remain cooperative-safe.