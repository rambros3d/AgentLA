## 1. PIO Wire Transports & Pin Sessions

- [x] 1.1 Implement `swd_probe.pio` microcode and `SwdTransport` driver in `firmware-rs`
- [x] 1.2 Implement `swio.pio` microcode and `SwioTransport` driver for 1-wire WCH targets
- [x] 1.3 Implement `rvswd.pio` microcode and `RvswdTransport` driver for 2-wire WCH targets
- [x] 1.4 Implement `ProgrammerPins` RAII session manager (`into_swd()`, `into_swio()`, `into_rvswd()`, `release()`)

## 2. CMSIS-DAP v1 & v2 Engine

- [x] 2.1 Implement unified `CmsisDapHandler` command parser (`DAP_Info`, `DAP_Connect`, `DAP_Disconnect`, `DAP_TransferConfigure`)
- [x] 2.2 Implement `DAP_Transfer` and `DAP_TransferBlock` AP/DP read/write loops over `SwdTransport`
- [x] 2.3 Implement `DAP_SWJ_Pins` and `DAP_SWJ_Sequence` raw bitstream clocking
- [x] 2.4 Wire `CmsisDapHandler` into `embassy-usb` HID endpoint (CMSIS-DAP v1) and WinUSB bulk endpoints (CMSIS-DAP v2)

## 3. Target Architecture Controllers

- [x] 3.1 Implement ARM Cortex-M controller (DHCSR halt/resume, DBGMCU IDCODE inspection, memory 32-bit read/write)
- [x] 3.2 Implement RISC-V QingKe controller (dmcontrol, dmstatus, DMI register access, Program Buffer execution)
- [x] 3.3 Implement target identification registry (STM32, RP2040/RP2350, CH32, PY32)

## 4. In-SRAM Algorithm Runner (`algo_runner`)

- [x] 4.1 Implement target SRAM memory downloader and register setup (`R0`-`R3`, `SP`, `PC`, `LR`)
- [x] 4.2 Implement synchronous execution monitor (polling `DHCSR` / `dmstatus` for `BKPT` with timeout)
- [x] 4.3 Integrate FLM blob runners for sector erase, page program, and memory verify

## 5. CLI, REST API & Staged Hardware Verification

- [x] 5.1 Implement serial REPL commands (`TARGET DETECT`, `TARGET HALT`, `TARGET RESUME`, `FLASH READ`)
- [x] 5.2 Implement REST API endpoints (`/api/v1/programmer/target`, `/api/v1/programmer/flash`) and SSE telemetry
- [x] 5.3 Verify STM32 target detection and flashing with `probe-rs` and REST API
- [x] 5.4 Verify RP2040 / RP2350 target detection and flashing
- [x] 5.5 Verify WCH CH32V003 (1-wire) and CH32V203 (2-wire) detection and flashing
- [x] 5.6 Verify Puya PY32F0 detection and flashing
