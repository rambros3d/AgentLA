## Why

In high-speed 2-wire debug protocols (ARM SWD and WCH RVSWD), line turnaround between the host probe and target microcontroller can lead to bus contention, ringing, and signal integrity degradation when bidirectional GPIO pins switch directions. The reference hardware in `ref/hybridlink` (Jolt Connect and Nucleo ST-Links) solves this by splitting SWDIO into a driver pin (driven through a series resistor) and a high-impedance sense pin, enabling instantaneous half-duplex line release without turnaround dead-cycles or bus fighting.

Adapting this split-pin wire transceiver topology into AgentLA's BSP and programmer crates allows hardware configurations to select either standard single-pin bidirectional routing (e.g. Raspberry Pi Pico 2) or split driver/sense resistor routing (e.g. BakuProbe / custom debug probes) with optimal signal fidelity.

## What Changes

- **Probe Wire Topology Abstraction**: Define `ProbeWireTopology` in `agentla-bsp` and `agentla-programmer` representing either `Combined { clk, dio }` or `Split { clk, dio_drv, dio_sns }`.
- **Zero-Contention Half-Duplex Operation**: When `Split` topology is active, driving is enabled on `dio_drv` for transmissions and tri-stated during receptions, while `dio_sns` continuously samples the target bus.
- **BSP Adapter Pins Update**: Add `probe` topology field to `AdapterPins` in `agentla-bsp` with defaults for supported boards.
- **Transport Compatibility**: Ensure both SWD and RVSWD transports can operate with either combined single-pin or split-pin configurations.

## Capabilities

### New Capabilities
- `split-pin-transceiver-topology`: Split driver/sense resistor network topology abstraction for instantaneous half-duplex turnaround and bus contention elimination.

### Modified Capabilities
<!-- None -->

## Impact

- **Crates**:
  - `crates/agentla-bsp`: Add `ProbeWireTopology` enum and update `AdapterPins` in `traits.rs`.
  - `crates/agentla-programmer`: Update `SwdIo` / `RvswdIo` traits or pin configuration structs to accept `ProbeWireTopology`.
- **Hardware Flexibility**: Allows AgentLA to run seamlessly on both standard breakout boards (single GPIO per signal) and dedicated hardware probes with resistor-separated driver/sense networks.
