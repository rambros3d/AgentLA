## Context

In Milestone 1, the clean-slate RP2350 bare-metal Embassy firmware was established in the `rust` branch with composite USB (Dual CDC, CMSIS-DAP scaffolding, CDC-NCM), virtual networking (`embassy-net`), and an asynchronous web server (`picoserve` + `littlefs2`).

Milestone 2 implements the debugging and target flashing core: low-level wire transports running on RP2350 PIO state machines, high-throughput CMSIS-DAP v1/v2 packet processing, and autonomous on-probe flashing with in-SRAM algorithm execution.

## Goals / Non-Goals

**Goals:**
- Implement the three primary hardware wire transports:
  - ARM SWD (ADIv5) via PIO microcode (`swd_probe.pio`) clocking up to 18.75 MHz.
  - WCH QingKe 1-Wire SWIO (`swio.pio`) running pulse-width modulated debug for CH32V003.
  - WCH QingKe 2-Wire RVSWD/SDI (`rvswd.pio`) for CH32V203/CH592.
- Provide a type-safe `ProgrammerPins` abstraction with RAII session transitions (`SwdSession`, `SwioSession`, `RvswdSession`) guaranteeing exclusive pin ownership.
- Implement the full CMSIS-DAP protocol suite (`DAP_Info`, `DAP_Connect`, `DAP_Transfer`, `DAP_TransferBlock`, `DAP_SWJ_Sequence`, `DAP_SWJ_Pins`).
- Implement an autonomous target detection and in-SRAM FLM algorithm runner (`algo_runner`).
- Verify target detection and flashing across 4 distinct target architectures:
  1. STMicroelectronics (STM32F103 / STM32F411 / STM32H5)
  2. Raspberry Pi (RP2040 / RP2350)
  3. WCH (CH32V003 1-Wire and CH32V203 2-Wire)
  4. Puya (PY32F002A / PY32F030)

**Non-Goals:**
- Implementing the 10-channel high-speed logic analyzer capture engine and DMA circular buffering (reserved for Milestone 3).
- Peripheral bus bridges (I2C/SPI master, SocketCAN gs_usb) (reserved for Milestone 4).

## Decisions

### 1. PIO Microcode Execution in Rust
- **Choice**: Embed PIO instructions via `pio-proc::pio_asm!` or pre-assembled `pio::Program` structs into `embassy-rp::pio`.
- **Rationale**: Guarantees bit-accurate parity with the reference C implementations (`swd_probe.pio`, `swio.pio`, `rvswd.pio`) while running at hardware speeds with zero CPU overhead for clock/data timing.

### 2. Synchronous Wire Transfers within Asynchronous Tasks
- **Choice**: Execute low-level wire transfers (reading/writing single 32-bit registers, ACK/WAIT polling) synchronously in bounded spin-loops (`< 1000 µs`), while keeping the outer USB packet handler, REST API dispatcher, and flash progress reporting asynchronous.
- **Rationale**: An SWD 32-bit transfer completes in 2–4 microseconds at 10 MHz. Yielding to an async executor for individual bit loops introduces microsecond scheduling jitter and severely degrades CMSIS-DAP throughput.

### 3. Shared Packet Engine for CMSIS-DAP v1 and v2
- **Choice**: A unified `CmsisDapHandler` implementing `fn handle_packet(&mut self, req: &[u8], resp: &mut [u8]) -> usize`.
- **Rationale**: Eliminates code duplication. CMSIS-DAP v1 (64-byte HID packets) and CMSIS-DAP v2 (up to 512-byte WinUSB bulk packets) share the exact same command dispatch and response formatting logic.

### 4. Modular Target Flashing & FLM Algo Runner
- **Choice**: Implement `trait TargetController` and an in-SRAM `FlashAlgorithmRunner` executing CMSIS-Pack FLM blobs (Thumb and RISC-V binaries) in target SRAM.
- **Rationale**: Matches AgentLA's proven C flashing engine. Target flash memory controllers are operated by standard Keil/CMSIS algorithm blobs loaded into target SRAM, bypassing slow register bitbanging.

## Risks / Trade-offs

- **[Risk] Bus Contention across Shared Pins**: `GPIO 4` and `GPIO 5` are physically shared across SWD, SWIO, and RVSWD.
  - *Mitigation*: The `ProgrammerPins` struct enforces exclusive ownership. Instantiating a `SwdSession` consumes the pins; switching to `SwioSession` requires dropping or calling `.release()`, ensuring the PIO state machine is disabled and pins are tristated first.
- **[Risk] Target Crash or Lockup during SRAM Algo Execution**: A target might hang in an infinite loop inside `Init()` or `ProgramPage()`.
  - *Mitigation*: The algorithm runner enforces a bounded timeout (e.g. 2000 ms). If the target fails to hit the breakpoint within the deadline, the probe halts the target, asserts `nRST`, and returns an explicit `AlgorithmTimeout` error.
