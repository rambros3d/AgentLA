## 1. BSP Topology Model

- [x] 1.1 Implement `ProbeWireTopology` enum in `crates/agentla-bsp/src/traits.rs` with `clk()`, `dio_out()`, `dio_in()`, and `is_split()` helper methods.
- [x] 1.2 Add `pub probe: Option<ProbeWireTopology>` field to `AdapterPins` in `crates/agentla-bsp/src/traits.rs`.
- [x] 1.3 Configure default probe wire topology on Pico 2 (`Combined`) and BakuProbe (`Split` or `Combined`).

## 2. Programmer Crate Integration

- [x] 2.1 Re-export `ProbeWireTopology` in `crates/agentla-programmer/src/traits.rs`.
- [x] 2.2 Add unit tests in `crates/agentla-bsp` and `crates/agentla-programmer` validating pin mapping logic for both `Combined` and `Split` topologies.

## 3. Verification & Testing

- [x] 3.1 Run unit tests with `cargo test --target x86_64-unknown-linux-gnu -p agentla-bsp -p agentla-programmer`.
- [x] 3.2 Verify embedded build with `cargo check --target thumbv8m.main-none-eabihf`.
