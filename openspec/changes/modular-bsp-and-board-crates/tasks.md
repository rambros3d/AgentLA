## 1. Core BSP Crate & Trait Definitions (`crates/agentla-bsp`)

- [x] 1.1 Create `crates/agentla-bsp` library crate with `no_std` `Cargo.toml` integrated into the Cargo workspace.
- [x] 1.2 Define `BoardProfile` trait, `ProgrammerPins`, `AdapterPins`, `FlexiblePool`, and `AnalogFrontendConfig` in `crates/agentla-bsp/src/traits.rs`.
- [x] 1.3 Implement reusable TI `Pga113` SPI driver (gain mapping 1x–200x, VCAL calibration) in `crates/agentla-bsp/src/drivers/pga113.rs`.
- [x] 1.4 Implement reusable TI `Ina180` current sensing helper in `crates/agentla-bsp/src/drivers/ina180.rs`.

## 2. Dedicated Board Crates (`boards/*`)

- [x] 2.1 Create `boards/pico2` board crate implementing `BoardProfile` for Raspberry Pi Pico 2 baseline (GPIO 0–29).
- [x] 2.2 Create `boards/bakuprobe` board crate implementing `BoardProfile` for BakuProbe (RP2350B, 48 pins, PGA113 front-end, INA180 current sense).
- [x] 2.3 Create `boards/templates` boilerplate crate as a reference for custom hardware developers.

## 3. Firmware Decoupling & Manifest Integration

- [x] 3.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-bsp` and active board crate `boards/pico2`.
- [x] 3.2 Refactor `firmware-rs` programmer, bus bridges, and instrument engines to source pins dynamically from `BoardProfile`.
- [x] 3.3 Update `/api/board/manifest` endpoint in `firmware-rs/src/web.rs` to generate manifest JSON from the active board profile.

## 4. Build, Flashing & Hardware Verification

- [x] 4.1 Recompile `firmware-rs` release build across workspace and generate UF2 binary.
- [x] 4.2 Flash physical RP2350 probe and verify zero regressions on `/api/board/manifest`, `/api/v1/programmer/*`, and `/api/v1/oscilloscope/*`.
- [x] 4.3 Git commit all artifacts cleanly on branch `embassy`.
