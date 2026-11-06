## Why

In Milestone 1, we established the bare-metal RP2350 Embassy runtime, composite USB interfaces, virtual networking, and asynchronous web server. To make AgentLA fully operational as an embedded debug probe and hardware bring-up lab, we now need the hardware debugging and flashing subsystem.

Porting the debugging and flashing engine to Rust Embassy provides memory safety, compile-time pin ownership semantics (preventing bus contention between multiplexed debug protocols), high-speed CMSIS-DAP v1/v2 execution, and autonomous on-probe target programming for ARM Cortex-M and WCH QingKe RISC-V microcontrollers.

## What Changes

- Implement PIO-accelerated wire transports in Rust:
  - ARM SWD (ADIv5) via `swd_probe.pio` supporting up to 18.75 MHz clocking.
  - WCH QingKe 1-Wire SWIO via `swio.pio` for single-wire CH32V003 debugging.
  - WCH QingKe 2-Wire RVSWD/SDI via `rvswd.pio` for CH32V203/CH592 debugging.
- Enforce strict compile-time pin ownership and RAII session switching across shared `GPIO 4-7`.
- Implement full CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (WinUSB Bulk) packet processing engine.
- Implement an autonomous on-probe target detection and in-SRAM FLM algorithm runner (`algo_runner`).
- Expose autonomous flashing and target detection via serial CLI and Web REST API (`/api/v1/programmer/*`).
- Stage verification across four target families: STM32, RP2040/RP2350, WCH CH32, and Puya PY32.

## Capabilities

### New Capabilities
- `rust-wire-transports`: Hardware-accelerated PIO wire transports for ARM SWD, WCH 1-Wire SWIO, and WCH 2-Wire RVSWD with RAII pin session switching on GPIO 4–7.
- `rust-cmsis-dap-engine`: High-speed CMSIS-DAP v1 (HID) and v2 (WinUSB Bulk) command dispatcher supporting DAP_Transfer, DAP_TransferBlock, and DAP_SWJ sequences.
- `rust-target-flasher`: Autonomous on-probe target MCU detection, register/memory inspection, and in-SRAM FLM flash algorithm runner for STM32, RP2040/RP2350, WCH CH32, and PY32.

### Modified Capabilities
None.

## Impact

- **Firmware (`../AgentLA-rust/firmware-rs`)**: Adds PIO assemblies, wire transport drivers, CMSIS-DAP packet handlers, and target flash loaders.
- **Hardware Pins**: Manages `GPIO 4` (CLK), `GPIO 5` (DIO), `GPIO 6` (nRST), and `GPIO 7` (BOOT0).
- **USB & Host Tools**: Full compatibility with `probe-rs`, `pyocd`, `openocd`, and embedded web flashing tools.
