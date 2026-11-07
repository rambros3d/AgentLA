## Why

Target programming on AgentLA has historically been limited to a small subset of microcontrollers (STM32F1 and WCH QingKe) inside `crates/agentla-programmer`. While commercial probes like ST-LINK/V3 are strictly locked to ST silicon, universal probes like Segger J-Link or `probe-rs` support thousands of chips but rely heavily on 20+ MB host drivers. 

By leveraging RP2350's generous SPI flash (4 MB on Pico 2, 16 MB on BakuProbe), we allocate a **250 KB firmware budget** (under 6.1% of Pico 2 flash) to a dedicated `#![no_std]` domain crate (`crates/agentla-targets`). This achieves **100% universal industry target coverage** directly in probe firmware: 20 parameterized hardware flash engines, ~30 embedded high-performance RAM loaders for exotic targets, and a compressed part-number matcher covering all ~10,000 commercial microcontroller part numbers across 200 distinct silicon families.

## What Changes

- **New Domain Crate `crates/agentla-targets`**: Dedicated no-std workspace crate encapsulating target identification, silicon family models, flash controller register sequencing, and flash algorithm runner traits under a 250 KB firmware budget.
- **20 Parameterized Hardware Flash Engines (~35 KB code)**:
  - STM32 Classic 16-bit (F1/F3/GD32/APM32/AT32/CH32F)
  - STM32 Sector-Based (F2/F4/F7)
  - STM32 Modern 64-bit (G0/G4/L0/L1/L4/L5/C0/U5/H5)
  - STM32 Dual-Bank High-Performance (H7)
  - Puya PY32 Ultra-Low Power
  - Nordic NVMC (nRF51, nRF52, nRF5340 App/Net, nRF9160)
  - Microchip SAMD/C/L (SAMD10/11/20/21, SAMC20/21, SAML21)
  - Microchip SAME/V/PIC32CM (SAME51/E54, SAMV71)
  - NXP LPC Command-Based (LPC800, LPC1100, LPC54000)
  - NXP Kinetis FTFL/FTFA (KL25, KL26, K64F)
  - Texas Instruments MSPM0 & SimpleLink (CC13x2, CC26x2, TM4C)
  - Silicon Labs MSC (EFM32 Gecko, EFR32 Wireless)
  - Renesas RA Family (RA2, RA4, RA6)
  - Raspberry Pi ROM Execution Engine (RP2040, RP2350 ARM & Hazard3)
  - WCH QingKe V2 1-Wire Fast Flash (CH32V003)
  - WCH QingKe V4 2-Wire Fast Flash (CH32V103, V203, V307)
  - WCH BLE Fast Flash (CH571, CH573, CH582, CH592)
  - Espressif RISC-V Direct SPI Flash (ESP32-C2/C3/C6/H2)
  - GigaDevice RISC-V (GD32VF103)
  - Multi-vendor Hardware Security & Unbricking Engine (STM32 RDP0/1/2 regress, Nordic CTRL-AP APPROTECT, NXP MDM-AP mass erase)
- **Compressed Target Catalog & Radix Matcher (~80 KB const data)**:
  - Deduplicated silicon family table (200 distinct dies).
  - Compressed prefix/suffix radix trie matching all 10,000+ commercial part numbers to silicon descriptors.
- **Embedded RAM FLM Loaders for Exotic Targets (~90 KB const data)**:
  - Precompiled, position-independent RAM bytecode for complex multi-bank ECC/HSM silicon (STM32H7, LPC55S69, Renesas RA6).
- **3-Tier Execution Pipeline**: Automatic resolution cascade: Tier 1 (Direct Engine & Embedded ROM Loaders) -> Tier 2 (LittleFS `/system/algo/<target>.bin`) -> Tier 3 (Dynamic Host FLM streaming over REST/MCP).
- **Workspace Refactor**: Pure wire protocols (SWD, RVSWD, SWIO, CMSIS-DAP) remain in `agentla-programmer`, while all target knowledge moves to `agentla-targets`.

## Capabilities

### New Capabilities
- `domain-crate-targets`: Dedicated no-std workspace crate providing 20 direct engines, embedded RAM loaders, compressed 10,000-part catalog, and the 3-tier cascade within a 250 KB budget.

### Modified Capabilities
- `universal-target-detection`: Upgrades target detection from basic DBGMCU checking to compressed radix trie identification matching all 10,000+ commercial ARM Cortex-M and RISC-V parts.
- `flash-algorithms`: Updates flash algorithm execution to follow the 3-tier cascade with ROM-embedded high-performance FLM fallback for exotic targets.

## Impact

- **Affected Crates**:
  - `Cargo.toml`: Adds `crates/agentla-targets` to workspace members.
  - `crates/agentla-targets`: Newly created crate with 250 KB memory allocation.
  - `crates/agentla-programmer`: Target definitions refactored to consume `agentla-targets`.
  - `crates/agentla-mcp`: Adds full-catalog target lookup and multi-vendor unbricking diagnostics.
  - `firmware-rs`: Exposes `/api/v1/programmer/target` and `/api/v1/programmer/flash` backed by the universal catalog.
- **APIs**:
  - REST API `/api/v1/programmer/target` returns accurate commercial part numbers, silicon family, flash/RAM boundaries, and security states.
  - REST API `/api/v1/programmer/unbrick` triggers hardware mass-erase/RDP regress across STM32, Nordic, NXP, and SAMD.
  - MCP tool `mcp_programmer_write` programs any commercial microcontroller out of the box.
