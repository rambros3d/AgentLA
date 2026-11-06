## 1. Implement Auto-BOOTSEL in Rust Firmware

- [x] 1.1 Add `bootsel` command handling to serial REPL in `firmware-rs/src/usb.rs`
- [x] 1.2 Add `POST /api/v1/system/bootsel` endpoint to `firmware-rs/src/web.rs`
- [x] 1.3 Compile release binary and convert to `/tmp/firmware-rs.uf2` using `picotool`

## 2. Deploy Rust Firmware to AgentLA Probe

- [x] 2.1 Send `BOOTSEL\n` to connected AgentLA probe on `/dev/ttyACM1` to trigger hands-free BOOTSEL
- [x] 2.2 Verify device appears in RP2350 BOOTSEL mode via `picotool info`
- [x] 2.3 Flash `/tmp/firmware-rs.uf2` onto AgentLA probe using `picotool load -x`

## 3. Verify Rust Firmware & Autonomous Target Operation

- [x] 3.1 Verify probe reboots and enumerates as `AgentLA RP2350 Probe` on USB
- [x] 3.2 Test serial REPL commands (`status`, `identify`, `help`) on Rust firmware
- [x] 3.3 Test autonomous target detection on connected targets (Port 1 STM32 BluePill, Port 2 CH32V203)
- [x] 3.4 Verify hands-free `bootsel` command functions on the new Rust firmware
- [x] 3.5 Verify flash read, core halt, and core resume operations across target MCUs
