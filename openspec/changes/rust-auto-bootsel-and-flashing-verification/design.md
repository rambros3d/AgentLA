## Context

The user has connected an AgentLA probe (`Bus 003 Device 011: ID 1209:414d`) running legacy C firmware, with a Raspberry Pi Pico 2 (RP2350) wired to its programmer pins. We need to deploy the Rust Embassy firmware onto the probe hands-free and test target flashing.

## Goals / Non-Goals

**Goals:**
- Implement software BOOTSEL entry in `firmware-rs` via serial REPL (`bootsel`) and REST API (`POST /api/v1/system/bootsel`).
- Recompile the release binary into an RP2350 UF2 image (`firmware-rs.uf2`).
- Trigger hands-free BOOTSEL on the live AgentLA probe by sending `BOOTSEL\n` to `/dev/ttyACM0`.
- Flash the probe with `picotool load -x firmware-rs.uf2`.
- Verify the probe reboots, enumerates as `AgentLA RP2350 Probe`, responds to the new `bootsel` command, and detects the connected Pico 2 target over SWD.

**Non-Goals:**
- Rewiring hardware pins or modifying physical boards.

## Decisions

### 1. Bootrom Call for Auto-BOOTSEL
- **Choice**: Call `embassy_rp::rom_data::reset_to_usb_boot(0, 0)` in `src/usb.rs` and `src/web.rs`.
- **Rationale**: This is the official RP2350 bootrom entry point, cleanly halting execution and remounting the USB controller in bootrom BOOTSEL mode.

### 2. Hands-Free Probe Flash Sequence
1. Connect to `/dev/ttyACM0` and send `BOOTSEL\n`.
2. Wait 1 second for USB device re-enumeration in BOOTSEL mode.
3. Flash using `picotool load -x /tmp/firmware-rs.uf2`.
4. Verify new `/dev/ttyACM0` and `/dev/ttyACM1` serial ports appear.

## Risks / Trade-offs

- **[Risk] Serial terminal lock during reset**: Once `reset_to_usb_boot` runs, the USB connection drops immediately.
  - *Mitigation*: Flush UART / write confirmation packet before yielding to `reset_to_usb_boot(0, 0)`.
