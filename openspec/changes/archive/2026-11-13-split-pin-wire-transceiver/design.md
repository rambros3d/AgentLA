## Context

High-speed serial wire protocols (ARM SWD and WCH RVSWD) run half-duplex over a clock line (SWCLK) and a single bidirectional data line (SWDIO/DIO). In conventional microcontroller pinouts (such as the standard Raspberry Pi Pico 2), a single GPIO pin is dynamically reconfigured between push-pull output and high-impedance input. However, during protocol turnaround cycles, output disable latency and capacitance can cause brief bus fighting or signal degradation.

In reference hardware such as the Jolt Connect ([`ref/hybridlink/boards/jolt_connect/board.h`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/ref/hybridlink/boards/jolt_connect/board.h)), the probe uses a split-pin resistor network:
- `SWCLK`: Push-pull clock output.
- `SWDIO_DRV`: Driver output connected to the target line through a current-limiting series resistor (e.g. 100 ohm).
- `SWDIO_SNS`: Sense input connected directly to the target line without series resistance.

When reading from the target, the probe simply tri-states `SWDIO_DRV` and samples `SWDIO_SNS`. This eliminates turnaround dead-cycles, avoids bus contention, and protects the probe against conflicting target drive states.

## Goals / Non-Goals

**Goals:**
- Define `ProbeWireTopology` in `agentla-bsp` and `agentla-programmer` with support for both `Combined` and `Split` pin routing.
- Provide uniform accessors (`clk()`, `dio_out()`, `dio_in()`, `is_split()`) across both architectures.
- Integrate `ProbeWireTopology` into `AdapterPins` in `agentla-bsp`.
- Support seamless pin allocation in `agentla-pin-arbiter` ensuring both driver and sense pins are claimed when operating in split mode.

**Non-Goals:**
- Requiring physical resistor networks on boards that have direct single-pin wiring (the abstraction defaults gracefully to `Combined`).

## Decisions

### Decision 1: Unified `ProbeWireTopology` Representation

- **Choice**:
  ```rust
  #[derive(Debug, Clone, Copy, PartialEq, Eq)]
  pub enum ProbeWireTopology {
      Combined { clk: u32, dio: u32 },
      Split { clk: u32, dio_drv: u32, dio_sns: u32 },
  }
  ```
- **Rationale**:
  - `Combined` maps `dio_out()` and `dio_in()` to the same physical pin.
  - `Split` maps `dio_out()` to `dio_drv` and `dio_in()` to `dio_sns`.
  - Transport code can simply write to `dio_out()` and read from `dio_in()` without branching or duplicate state machines.

### Decision 2: Pin Arbiter Integration

- **Choice**: When claiming pins for the debug probe, if `ProbeWireTopology::Split` is configured, both `dio_drv` and `dio_sns` are claimed from the pin arbiter under the probe driver.
- **Rationale**: Prevents accidental reuse of the sense pin by other instruments or adapters.

## Risks / Trade-offs

- **[Risk] Pin consumption increase in split mode**
  - *Mitigation*: Split mode consumes 1 extra GPIO pin (3 pins total: CLK, DRV, SNS), which is easily accommodated on RP2350's 30/48 GPIO pads.
