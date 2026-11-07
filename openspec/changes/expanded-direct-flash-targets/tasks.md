## 1. Crate Scaffolding & Core Abstractions

- [x] 1.1 Create `crates/agentla-targets` with `Cargo.toml` and `#![no_std]` zero-heap configuration
- [x] 1.2 Add `crates/agentla-targets` to the root workspace `Cargo.toml` members
- [x] 1.3 Define `TargetMemAccess` trait, `TargetDescriptor`, `SiliconFamily`, and core error enums
- [x] 1.4 Setup CI / build verification script asserting crate size <= 250 KB

## 2. Direct Hardware Flash & Security Engines Implementation

- [x] 2.1 Implement STM32 family engines: `Stm32Classic16`, `Stm32Sector` (F2/F4/F7), `Stm32Modern64` (G0/G4/L4/C0/U5), and `Stm32DualBankH7`
- [x] 2.2 Implement Nordic family engine: `NordicNvmc` (nRF51, nRF52, nRF5340 App/Net, nRF9160)
- [x] 2.3 Implement Microchip family engines: `MicrochipSamd` (SAMD/C/L) and `MicrochipSame` (SAME51/E54, SAMV71, PIC32CM)
- [x] 2.4 Implement NXP family engines: `NxpLpc` (LPC800/1100/54000) and `NxpKinetis` (KL25/26, K64F FTFL/FTFA)
- [x] 2.5 Implement TI, Silicon Labs, and Renesas engines: `TiMspm0`, `SiliconLabsMsc`, and `RenesasRa`
- [x] 2.6 Implement Raspberry Pi and Puya engines: `RaspberryPiRom` (RP2040/RP2350 ROM calls) and `PuyaPy32`
- [x] 2.7 Implement RISC-V engines: `QingKeV2` (1-wire), `QingKeV4` (2-wire), `WchBle`, `EspressifRiscvSpi` (ESP32-C3/C6), and `GigaDeviceRiscv` (GD32VF103)
- [x] 2.8 Implement `HardwareSecurityRescue` unbricking engine (STM32 RDP0/1/2 regress, Nordic CTRL-AP APPROTECT, NXP MDM-AP mass erase)

## 3. Universal Target Catalog, Radix Matcher & Embedded Loaders

- [x] 3.1 Construct the 200 silicon family descriptor catalog with register bases, unlock keys, and sector layouts
- [x] 3.2 Build the compressed radix trie matcher resolving 10,000+ commercial part numbers into silicon family descriptors
- [x] 3.3 Embed precompiled position-independent RAM algorithm bytecode blobs for complex multi-bank targets (STM32H7, LPC55S69, Renesas RA6)
- [x] 3.4 Validate that code, tables, and embedded loaders strictly fit within the 250 KB budget

## 4. 3-Tier Resolution Cascade & Workspace Integration

- [x] 4.1 Implement 3-tier cascade dispatcher: Tier 1 (Direct & ROM Loaders) -> Tier 2 (LittleFS) -> Tier 3 (Host FLM)
- [x] 4.2 Update `crates/agentla-programmer` to integrate `agentla-targets` with SWD, RVSWD, and SWIO transports
- [x] 4.3 Update `firmware-rs` REST routes (`/api/v1/programmer/target`, `/api/v1/programmer/flash`, `/api/v1/programmer/unbrick`)
- [x] 4.4 Update `crates/agentla-mcp` tools for universal target discovery, unbricking, and diagnostics

## 5. Verification & Tests

- [x] 5.1 Implement mock-memory unit test suite in `crates/agentla-targets` verifying all 20 engines
- [x] 5.2 Validate radix matcher coverage across commercial part number test vectors on `x86_64`
- [x] 5.3 Verify live hardware flash read and write on STM32F1, CH32V203, and CH32V003 targets
