## Why

To support seamless autonomous operation and hands-free development without physical button presses, the Rust Embassy firmware must provide an auto-BOOTSEL reset command across both serial REPL and Web REST API. Furthermore, the AgentLA probe currently connected over USB must be transitioned hands-free to the new Rust Embassy firmware, and target detection and flashing must be verified against the connected Raspberry Pi Pico 2 (RP2350).

## What Changes

- Add hands-free `bootsel` command to `firmware-rs` serial REPL (`src/usb.rs`) invoking `embassy_rp::rom_data::reset_to_usb_boot(0, 0)`.
- Add `POST /api/v1/system/bootsel` endpoint to `firmware-rs` web server (`src/web.rs`).
- Build release UF2 image for RP2350 (`thumbv8m.main-none-eabihf`).
- Trigger hands-free reboot on the active AgentLA probe (`/dev/ttyACM0`) to enter RP2350 BOOTSEL mode without physical interaction.
- Deploy the Rust firmware onto the AgentLA probe using `picotool load -x`.
- Verify the Rust firmware enumerates, responds to `bootsel`, and autonomously detects the connected Raspberry Pi Pico 2 target over SWD.

## Capabilities

### New Capabilities
- `rust-auto-bootsel`: Software-controlled reboot to USB BOOTSEL mode via serial command `bootsel` and REST API `POST /api/v1/system/bootsel`.

### Modified Capabilities
- `rust-target-flasher`: Verified autonomous detection and communication with the Raspberry Pi Pico 2 (RP2350) target.

## Impact

- **Firmware (`/run/media/sun/halo/AgentLA-rust/firmware-rs`)**: Updates `src/usb.rs` and `src/web.rs`.
- **Physical Hardware**: Replaces C firmware on the connected AgentLA probe with the bare-metal Rust Embassy firmware.
- **Workflow**: Completely eliminates the need to press physical BOOTSEL buttons.
