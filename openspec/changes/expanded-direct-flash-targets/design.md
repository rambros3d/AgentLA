## Context

AgentLA currently implements physical wire protocols (SWD, RVSWD, SWIO) within `crates/agentla-programmer`, with hardcoded flash routines for only a small subset of microcontrollers (STM32F1 and WCH QingKe). While dedicated vendor probes like ST-LINK/V3 support hundreds of STM32 chips, universal probes like Segger J-Link and `probe-rs` support thousands of chips but rely heavily on 20+ MB host drivers.

By utilizing the RP2350's generous SPI flash (4 MB on Pico 2, 16 MB on BakuProbe), we allocate a **250 KB firmware budget** (under 6.1% of Pico 2 flash) to a dedicated `#![no_std]` domain crate `crates/agentla-targets`. This achieves **100% universal industry target coverage** directly in probe firmware: 20 parameterized hardware flash engines, ~30 embedded high-performance RAM loaders for complex targets, a compressed radix matcher resolving all 10,000+ commercial microcontroller parts across 200 distinct silicon families, and a multi-vendor unbricking engine.

## Goals / Non-Goals

**Goals:**
- Provide a dedicated, pure `#![no_std]` zero-heap crate `agentla-targets` in the workspace.
- Implement 20 parameterized direct flash engines covering major ARM Cortex-M and RISC-V silicon families (~35 KB code).
- Implement a multi-vendor hardware security and unbricking engine (STM32 RDP0/1/2 regress, Nordic CTRL-AP APPROTECT mass erase, NXP MDM-AP mass erase, SAMD security clear).
- Embed precompiled high-performance RAM algorithm bytecode blobs for complex multi-bank ECC targets (STM32H7, LPC55S69, Renesas RA6) (~90 KB const data).
- Maintain a deduplicated 200-family silicon catalog and a compressed radix matcher resolving all ~10,000 commercial part numbers (~80 KB const data).
- Total crate size strictly constrained under 250 KB (6.1% of 4 MB flash).
- Implement the 3-tier execution cascade: Tier 1 (Direct Engine & ROM-Embedded Loaders) -> Tier 2 (LittleFS `/system/algo/*.bin`) -> Tier 3 (Dynamic Host FLM stream).
- Abstract bus access through a generic `TargetMemAccess` trait (`read32`, `write32`, `read_block`, `write_block`).
- Enable full host unit testing on `x86_64` without compiling RP2350 PIO transports or Embassy runtimes.

**Non-Goals:**
- Moving physical wire transports (PIO state machines, line resets, turnaround timings) into `agentla-targets`; these remain in `agentla-programmer`.
- Replacing CMSIS-DAP v2 or GDB RSP interfaces; these continue to function alongside the direct engine.
- Storing uncompressed raw strings for all 10,000 part numbers; deduplication down to 200 silicon dies and radix trie prefix compression is required.

## Decisions

### Decision 1: Dedicated `agentla-targets` Crate
* **Rationale:** Decouples target silicon knowledge (register offsets, unlock keys, page geometries, IDCODEs) from physical wire transports (PIO bitbanging, SWD clocks, DMI parity).
* **Alternatives Considered:** Keeping inside `agentla-programmer` rejected due to build time inflation, lack of isolation, and difficulty testing on PC.

### Decision 2: 250 KB Firmware Budget Allocation
```
                    250 KB FIRMWARE BUDGET ALLOCATION
 ┌───────────────────────────────────────────────┬────────────────────────────┐
 │ Component                                     │ Size Budget                │
 ├───────────────────────────────────────────────┼────────────────────────────┤
 │ 20 Parameterized Flash Engines                │ ~35 KB code                │
 │ Hardware Security & Unbricking Engine         │ ~15 KB code                │
 │ 200 Silicon Family Descriptors Table          │ ~16 KB const data          │
 │ Compressed Part# Radix Matcher (10,000 parts) │ ~80 KB const data          │
 │ Precompiled High-Performance RAM Loaders      │ ~90 KB const data          │
 │ Framework Glue, Traits, Dispatcher            │ ~14 KB code                │
 ├───────────────────────────────────────────────┼────────────────────────────┤
 │ Total Crate Footprint                         │ <= 250 KB (6.1% of 4 MB)   │
 └───────────────────────────────────────────────┴────────────────────────────┘
```

### Decision 3: 20 Parameterized Hardware Flash Engines
1. `Stm32Classic16`: STM32 F1/F3, GD32F1/F3, APM32, AT32 (16-bit halfwords)
2. `Stm32Sector`: STM32 F2, F4, F7 (Sector-based 16KB/64KB/128KB, 32-bit parallelism)
3. `Stm32Modern64`: STM32 G0, G4, L0, L1, L4, L5, C0, U5, H5 (64-bit double-words)
4. `Stm32DualBankH7`: STM32H7 dual-bank 2 MB flash controller
5. `PuyaPy32`: Puya PY32F0 (128-byte sectors)
6. `NordicNvmc`: nRF51, nRF52, nRF5340 App/Net, nRF9160 (NVMC word writes)
7. `MicrochipSamd`: SAMD10/11/20/21, SAMC20/21, SAML21 (NVMCTRL rows)
8. `MicrochipSame`: SAME51/E54, SAMV71, PIC32CM
9. `NxpLpc`: NXP LPC800, LPC1100, LPC54000 command-based flash controller
10. `NxpKinetis`: NXP Kinetis KL25, KL26, K64F FTFL/FTFA controller
11. `TiMspm0`: Texas Instruments MSPM0, CC13x2, CC26x2 flash controller
12. `SiliconLabsMsc`: EFM32 Gecko, EFR32 Wireless Gecko Memory System Controller
13. `RenesasRa`: Renesas RA2, RA4, RA6 Cortex-M flash controller
14. `RaspberryPiRom`: RP2040, RP2350 boot ROM call execution
15. `QingKeV2`: CH32V003 1-wire SWIO 64-byte fast page autoexec
16. `QingKeV4`: CH32V103, V203, V307 2-wire RVSWD fast flash
17. `WchBle`: CH571, CH573, CH582, CH592 fast flash
18. `EspressifRiscvSpi`: ESP32-C2/C3/C6/H2 direct SPI flash controller
19. `GigaDeviceRiscv`: GD32VF103 RISC-V flash controller
20. `HardwareSecurityRescue`: Multi-vendor unbricking engine (STM32 RDP0/1/2 regress, Nordic CTRL-AP APPROTECT, NXP MDM-AP erase)

### Decision 4: 3-Tier Resolution Cascade
* **Tier 1 (Direct Built-in & ROM Loaders):** Sub-millisecond flash execution or ROM-embedded RAM loader execution, zero LittleFS access.
* **Tier 2 (LittleFS `/system/algo/`):** Autonomous on-probe storage of extended CMSIS-Pack FLM `.bin` files without firmware recompile.
* **Tier 3 (Host Dynamic Stream):** Zero-touch fallback for custom ASICs streamed over REST/MCP.

### Decision 5: Abstract `TargetMemAccess` Trait
```rust
pub trait TargetMemAccess {
    type Error;
    fn read32(&mut self, addr: u32) -> Result<u32, Self::Error>;
    fn write32(&mut self, addr: u32, val: u32) -> Result<(), Self::Error>;
    fn read_block(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), Self::Error>;
    fn write_block(&mut self, addr: u32, data: &[u8]) -> Result<(), Self::Error>;
}
```

## Risks / Trade-offs

- **[Risk] Crate size growth exceeds 250 KB**
  → *Mitigation:* Strict binary size assertions in CI, shared helper routines across related engines, radix trie compression for part names, and optional feature flags per vendor.
- **[Risk] High-end targets with complex ECC caches failing direct writes**
  → *Mitigation:* Complex targets (STM32H7, LPC55S, RA6) route through precompiled, validated ROM-embedded FLM loaders rather than raw register bitbanging.
- **[Risk] Chip misidentification during mass erase**
  → *Mitigation:* Dual validation requiring CPUID/DBGMCU match and factory flash size boundary verification before unlock keys are asserted.
