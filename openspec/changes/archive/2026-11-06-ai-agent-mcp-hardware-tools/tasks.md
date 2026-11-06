## 1. Electrical Diagnostics & Physical Layer

- [x] 1.1 Implement target rail voltage sensing via ADC in `firmware-rs/src/programmer/target/diagnostic.rs`
- [x] 1.2 Implement dynamic pad pull-down perturbation test (driven vs. pulled-up vs. floating vs. short)
- [x] 1.3 Add unit tests in `test/test_mcp_electrical_diag.py`

## 2. CoreSight Fault Analysis & One-Shot Diagnostic (`mcp_target_diagnose`)

- [x] 2.1 Implement universal ARM CoreSight SCB reader (`DHCSR`, `CFSR`, `HFSR`, `BFAR`, `PC`, `LR`) and RISC-V DM fallback
- [x] 2.2 Construct structured diagnostic JSON report with root cause synthesis
- [x] 2.3 Wire `mcp_target_diagnose` into `firmware-rs/src/mcp.rs` tool dispatcher
- [x] 2.4 Add unit tests in `test/test_mcp_target_diagnose.py`

## 3. Firmware Vector Table & Layout Inspector (`mcp_inspect_firmware`)

- [x] 3.1 Implement flash vector table parser (SP, Reset, NMI, HardFault)
- [x] 3.2 Implement dynamic stack pointer boundary validation using LittleFS `AlgoBlobHeader`
- [x] 3.3 Implement ARM Thumb mode invariant check (`(Reset & 1) == 1`) and option byte reading
- [x] 3.4 Wire `mcp_inspect_firmware` into `firmware-rs/src/mcp.rs` tool dispatcher
- [x] 3.5 Add unit tests in `test/test_mcp_inspect_firmware.py`

## 4. Autonomous Bus Sniffer (`mcp_sniff_bus`)

- [x] 4.1 Implement time-windowed logic analyzer capture loop for I2C and SPI in `analyzer.rs`
- [x] 4.2 Stream captured samples into in-firmware decoders and aggregate transaction JSON
- [x] 4.3 Wire `mcp_sniff_bus` into `firmware-rs/src/mcp.rs` tool dispatcher
- [x] 4.4 Add unit tests in `test/test_mcp_sniff_bus.py`
