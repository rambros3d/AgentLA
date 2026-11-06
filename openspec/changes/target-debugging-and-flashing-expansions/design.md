## Context

AgentLA functions as an autonomous, multi-protocol hardware debug probe, logic analyzer, and test station powered by the Raspberry Pi RP2350 microcontroller. While its SWD engine can read, write, and flash supported Cortex-M and QingKe RISC-V targets, three critical operational bottlenecks exist:
1. **Target Fault Lockups & SWD Disconnects**: When target firmware crashes (e.g. invalid stack pointer, HardFault loops) or powers down debug clocks (`WFI` / `WFE` deep sleep), the SWD debug port becomes unresponsive. Because the probe previously attached while the target ran freely, targets frequently wedged before halt could be established (`PC = 0xFFFFFFFE` fault).
2. **Bricked Target Recovery**: When targets are locked with ST Readout Protection (RDP Level 1) or when firmware repurposes SWD pins (PA13/PA14) as GPIOs, SWD access is locked out. Salvaging these microcontrollers requires vendor UART bootloader orchestration via `BOOT0` and `nRST`.
3. **Flash Algorithm Scalability**: Expanding flash support historically required hardcoding flash controller register routines into the probe firmware ROM. Keil CMSIS-Pack `.FLM` executables define a universal standard, but were not integrated with AgentLA's storage subsystem.

With the recently deployed **Dual LittleFS Partition architecture** (Partition 1 `SYSTEM_FS` with 382 KB free headroom), AgentLA now has the storage capacity to decouple flash algorithm binaries from probe ROM.

---

## Goals / Non-Goals

### Goals
- **Connect-under-Reset Guarantee**: Guarantee that Cortex-M targets halt on cycle 0 of the Reset Vector (`DEMCR.VC_CORERESET`) before executing buggy firmware or entering low-power modes.
- **Automated STM32 Unbricking**: Provide a single-click/one-API recovery command (`POST /api/v1/programmer/rescue/stm32`) that toggles `BOOT0` and `nRST`, syncs via AN3155 protocol, and executes Readout Unprotect (`0x92`) mass-erase to revive bricked silicon.
- **Filesystem-Backed Flash Algorithms**: Allow `FlashAlgorithmRunner` to read standardized binary algorithm blobs from LittleFS Partition 1 (`/system/algo/<family>.bin`), eliminating chip-specific flash drivers from the core firmware ROM.
- **Host Packager Pipeline**: Provide a Python tool (`tools/pack_flm.py`) that extracts Thumb code and function entry points (`Init`, `EraseSector`, `ProgramPage`, `UnInit`) from Keil/CMSIS `.FLM` or ELF binaries into the compact `AlgoBlobHeader` format.

### Non-Goals
- On-probe ELF parsing at runtime (parsing raw ELF headers on the MCU adds unnecessary RAM overhead and dependencies; host packager extracts binary blobs ahead of time).
- Supporting non-ARM architectures with CMSIS-Pack FLM (FLM is specifically ARM Cortex-M Thumb bytecode; WCH RISC-V uses native QingKe ProgBuf).
- WCH single-wire bootloader support in this iteration (focusing on STM32 AN3155 first).

---

## Decisions

### 1. Vector Catch & Reset Sequencing
* **Decision**: Implement two reset modes in `CortexM`:
  1. `connect_under_reset()`: Drives physical `PIN_RESET` LOW, initializes SWD line sync (50+ clocks high, JTAG-to-SWD sequence `0xE79E`), sets `DEMCR.TRCENA` (bit 24) and `DEMCR.VC_CORERESET` (bit 0) via SWD, then releases `PIN_RESET` and polls `DHCSR.S_HALT`.
  2. `reset_and_halt()`: Uses `AIRCR.SYSRESETREQ` (`0x05FA0004`) while `DEMCR.VC_CORERESET` remains armed, allowing software resets when target `nRST` is not physically connected.
* **Rationale**: Halting on cycle 0 is the industry standard (used by OpenOCD, J-Link, and PyOCD) to guarantee predictable debug access regardless of target flash contents.

### 2. Dedicated Hardware Bootloader Orchestration (ST AN3155)
* **Decision**: Implement `Stm32Bootloader` state machine operating over AgentLA's dedicated programmer pins:
  * Assert `PIN_BOOT0` = HIGH (GPIO 2).
  * Pulse `PIN_RESET` = LOW for 20 ms, then HIGH (GPIO 6/10/13).
  * Wait 50 ms for bootloader initialization.
  * Switch UART0 (GPIO 8 TX / 14 RX) to `8E1` parity at 115200 baud.
  * Transmit `0x7F` auto-baud byte, expecting `0x79` (ACK).
  * Support `Readout Unprotect` command (`0x92` + inverted byte `0x6D`), which triggers automatic full chip mass-erase and resets RDP to Level 0.
  * Release `PIN_BOOT0` = LOW upon completion.
* **Rationale**: Reuses existing `ProgrammerPins` and `bridge/uart.rs` hardware without extra hardware modifications.

### 3. Flash Algorithm Storage & Execution ABI
* **Decision**: Flash algorithm blobs are stored as flat files on LittleFS Partition 1: `/system/algo/<chip>.bin`.
  * The file begins with a 48-byte `AlgoBlobHeader`:
    ```rust
    #[repr(C)]
    pub struct AlgoBlobHeader {
        pub magic: u32,          // "ALGO" (0x4F474C41)
        pub version: u32,        // 1
        pub code_size: u32,      // Length of Thumb code in bytes
        pub init_offset: u32,    // Offset of Init(adr, clk, fnc)
        pub erase_offset: u32,   // Offset of EraseSector(adr)
        pub program_offset: u32, // Offset of ProgramPage(adr, sz, buf)
        pub verify_offset: u32,  // Offset of Verify(adr, sz, buf)
        pub uninit_offset: u32,  // Offset of UnInit(fnc)
        pub ram_base: u32,       // Target SRAM address to place blob
        pub ram_size: u32,       // SRAM working size
        pub stack_top: u32,      // Target initial stack pointer
    }
    ```
  * `FlashAlgorithmRunner` downloads the code into target SRAM, sets R0–R3 according to the CMSIS FLM calling convention, sets PC to the function entry, sets LR to a breakpoint address, and resumes the core. Upon hitting `BKPT 0`, it verifies R0 == 0.
* **Rationale**: Completely separates device support from firmware binaries. Adding new chip families requires only copying a 2–4 KB binary file into `/system/algo/` via WebUI or LittleFS image builder.

---

## Risks / Trade-offs

| Risk | Mitigation |
| :--- | :--- |
| **Target lacks physical `nRST` line** | Connect-under-reset falls back to vector catch armed before triggering software `SYSRESETREQ` via `AIRCR`. |
| **Target power rail dip on mass-erase** | ST AN3155 Readout Unprotect can take up to 2 seconds for flash mass-erase; timeout in UART driver set to 5000 ms with polling. |
| **LittleFS algorithm blob corruption** | Header includes magic `0x4F474C41`, version check, and code size bound checks; operations reject with `ERR ALGO INVALID` if verification fails. |
| **UART Pin Conflicts with Logic Analyzer** | Bootloader engine claims pins through `PinArbiter`; if UART pins are claimed by logic analyzer, returns `409 Conflict`. |
