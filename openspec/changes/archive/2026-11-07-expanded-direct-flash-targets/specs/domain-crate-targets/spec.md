## ADDED Requirements

### Requirement: Dedicated no-std targets crate with 250 KB budget
The system SHALL provide a dedicated `#![no_std]` crate `agentla-targets` operating within a strict 250 KB firmware budget, encapsulating target identification, silicon family definitions, 20 direct flash engines, embedded high-performance RAM loaders, and a compressed radix matcher.

#### Scenario: Crate compilation in no-std
- **WHEN** `cargo check --target thumbv8m.main-none-eabihf -p agentla-targets` is executed
- **THEN** the crate compiles without the standard library and with zero heap allocation.

### Requirement: Twenty parameterized direct flash engines
The crate SHALL implement 20 parameterized hardware flash and security engines:
1. `Stm32Classic16`: STM32 F1/F3, GD32F1/F3, APM32, AT32, CH32F 16-bit halfword programming.
2. `Stm32Sector`: STM32 F2, F4, F7 sector-based 32-bit programming.
3. `Stm32Modern64`: STM32 G0, G4, L0, L1, L4, L5, C0, U5, H5 64-bit double-word programming.
4. `Stm32DualBankH7`: STM32H7 dual-bank 2 MB flash controller.
5. `PuyaPy32`: Puya PY32F0 128-byte sector controller.
6. `NordicNvmc`: nRF51, nRF52, nRF5340 App/Net, nRF9160 direct NVMC controller.
7. `MicrochipSamd`: SAMD10/11/20/21, SAMC20/21, SAML21 NVMCTRL controller.
8. `MicrochipSame`: SAME51/E54, SAMV71, PIC32CM NVMCTRL controller.
9. `NxpLpc`: NXP LPC800, LPC1100, LPC54000 command-based flash controller.
10. `NxpKinetis`: NXP Kinetis KL25, KL26, K64F FTFL/FTFA controller.
11. `TiMspm0`: Texas Instruments MSPM0, CC13x2, CC26x2 flash controller.
12. `SiliconLabsMsc`: EFM32 Gecko, EFR32 Wireless Gecko Memory System Controller.
13. `RenesasRa`: Renesas RA2, RA4, RA6 Cortex-M flash controller.
14. `RaspberryPiRom`: RP2040 and RP2350 boot ROM call execution engine.
15. `QingKeV2`: CH32V003 1-wire SWIO 64-byte fast page autoexec engine.
16. `QingKeV4`: CH32V103, CH32V203, CH32V307 2-wire RVSWD fast flash engine.
17. `WchBle`: CH571, CH573, CH582, CH592 fast flash engine.
18. `EspressifRiscvSpi`: ESP32-C2/C3/C6/H2 direct SPI flash controller engine.
19. `GigaDeviceRiscv`: GD32VF103 RISC-V flash controller engine.
20. `HardwareSecurityRescue`: Multi-vendor unbricking engine (STM32 RDP0/1/2 regress, Nordic CTRL-AP APPROTECT mass erase, NXP MDM-AP erase).

#### Scenario: Direct flash write execution
- **WHEN** `flash_write` is called on a recognized target descriptor
- **THEN** the assigned direct engine unlocks the target's flash controller, erases sectors, writes pages, and verifies without downloading RAM code.

#### Scenario: Hardware unbrick execution
- **WHEN** target unbrick is requested on a locked microcontroller
- **THEN** the `HardwareSecurityRescue` engine asserts core reset and issues vendor-specific debug-port mass erase commands.

### Requirement: Universal 10,000-part compressed catalog
The crate SHALL store a deduplicated 200-family silicon table and a compressed radix matcher resolving over 10,000 commercial part numbers, consuming no more than 80 KB of const data flash.

#### Scenario: Part number resolution
- **WHEN** an IDCODE, DevID, or commercial part name string is queried
- **THEN** the matcher resolves to the exact silicon family descriptor with flash base, size, sector geometry, and engine kind in $O(\log N)$ time.

### Requirement: Precompiled embedded RAM loaders
The crate SHALL store precompiled, position-independent RAM algorithm bytecode blobs for complex multi-bank ECC targets (STM32H7, LPC55S69, Renesas RA6) within a 90 KB budget.

#### Scenario: Embedded RAM loader execution
- **WHEN** a complex target requiring RAM execution is flashed and LittleFS is unmounted
- **THEN** the runner loads the precompiled ROM-embedded FLM blob into target SRAM and executes `ProgramPage` successfully.
