## 1. RVSWD Protocol & RISC-V Debug Module

- [x] 1.1 Update `crates/agentla-programmer/src/rvswd.rs` with double even parity, 4 padding zeros, and target status nibble `0111` verification matching `ref/hybridlink/app/rvswd.c`.
- [x] 1.2 Implement `crates/agentla-programmer/src/rvdm.rs` for RISC-V 0.13 Debug Module commands, abstract registers, and `rvdm_mem_stream32` program-buffer streaming.
- [x] 1.3 Add unit tests in `crates/agentla-programmer` for RVSWD packet framing, parity calculations, and DMI status decoding.

## 2. Protocol Parsers in agentla-programmer

- [x] 2.1 Implement `crates/agentla-programmer/src/stlink.rs` parsing ST-Link `F1` (version `V2J48M35`), `F2` (SWD core and AP memory read/write), `F3` (exit), and `F5` (mode query).
- [x] 2.2 Implement `crates/agentla-programmer/src/wch.rs` parsing WCH-Link `0D` (version 2.12, attach chip ID), `0C` (clock speed), `08` (DMI read/write), `11` (chip info), and `01`/`02` (flash write/verify).
- [x] 2.3 Export `StLinkHandler` and `WchLinkHandler` in `crates/agentla-programmer/src/lib.rs`.

## 3. Dynamic USB Descriptors and Re-enumeration

- [x] 3.1 Define `ProgrammerPersonality` enum (`CmsisDap`, `StLink`, `WchLink`) with persistence support in `firmware-rs`.
- [x] 3.2 Update `firmware-rs/src/usb.rs` to construct composite descriptors dynamically per personality while retaining CDC-NCM, CDC-ACM, `gs_usb`, and `i2c-tiny-usb`.
- [x] 3.3 Implement async task runners `stlink_usb_task` and `wchlink_usb_task` in `firmware-rs/src/usb.rs`.
- [x] 3.4 Implement USB soft-detach and re-enumeration sequence on personality change.

## 4. REST API & WebUI Controls

- [x] 4.1 Implement `GET /api/programmer/mode` and `POST /api/programmer/mode` endpoints in `firmware-rs/src/rest.rs`.
- [x] 4.2 Add programmer personality selection UI controls to the web dashboard.

## 5. Verification & Testing

- [x] 5.1 Verify `#![no_std]` build with `cargo check --target thumbv8m.main-none-eabihf` on `firmware-rs`.
- [x] 5.2 Run unit tests with `cargo test -p agentla-programmer`.
