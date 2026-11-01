## Context

AgentLA organizes RP2350 GPIOs into three 10-pin domains: `PROGRAMMER` (GPIO 0-9), `ADAPTER` (GPIO 10-19), and `LOGIC ANALYZER` (GPIO 20-29). The probe currently supports I2C, SPI, UART, and CAN within the Adapter domain. 

The reference `I3CBlaster` project implements an I3C controller on RP2040 using a PIO state machine. However, `I3CBlaster` assumes a standalone single-purpose firmware where `PIO0` instruction memory is completely overwritten at runtime, hardcodes a 125 MHz system clock, and uses Seeed Xiao or Pico board pinouts.

To integrate I3C into AgentLA without disrupting CMSIS-DAP/SWD on `PIO0` or Logic Analyzer capture on `PIO1`, we must assign I3C to the unused `PIO2` on the RP2350, adapt the clock divider to arbitrary system frequencies, integrate with `pin_owner` on Adapter pins (GPIO 10/11), and provide end-to-end verification against real silicon using an STM32H503 microcontroller.

## Goals / Non-Goals

**Goals:**
- Implement a robust MIPI I3C Master Controller engine in `firmware/src/adapter/adapter_i3c.c` based on `ref/I3CBlaster/src/i3c.pio`.
- Allocate `PIO2` on RP2350 for the I3C engine to maintain full concurrency with SWD (`PIO0`) and the Logic Analyzer (`PIO1`).
- Dynamically compute PIO clock dividers using `clock_get_hz(clk_sys)` to support standard 100 kHz, 400 kHz, 1 MHz, 4 MHz, and 12.5 MHz I3C clocking.
- Implement Dynamic Address Assignment (ENTDAA), Common Command Codes (CCC), private SDR read/write, and In-Band Interrupt (IBI) polling.
- Expose I3C operations via serial CLI and the Web UI command bridge.
- Build an end-to-end automated verification suite on STM32H503 hardware: flash target firmware via SWD, configure STM32H503's native I3C peripheral as target, and validate DAA, private SDR transfers, and IBI.

**Non-Goals:**
- Tertiary multi-lane I3C modes (e.g. TSP/TSL) or HDR-BT modes (I3C Basic spec limits to SDR and HDR-DDR).
- On-board active hardware level-shifting on the current probe PCB (users testing at 1.0V/1.2V/1.8V will use external level translation, or run 3.3V I3C mode directly supported by the STM32H503).
- Rewriting the Logic Analyzer PIO capture engine.

## Decisions

### Decision 1: Assign I3C Controller to RP2350 `PIO2`
- **Choice**: Use RP2350 `PIO2` exclusively for the I3C state machine.
- **Rationale**: `I3CBlaster` dynamically swaps PIO programs in instruction memory between SDR (`i3c`), T-bit overlay (`i3c_overlay`), and HDR-DDR (`i3c_ddr`). Overwriting `PIO0` would corrupt the active CMSIS-DAP / SWD debug probe program (`swd_probe.pio`). Since `PIO2` is unallocated in AgentLA, isolating I3C on `PIO2` prevents instruction conflicts and guarantees zero interference with SWD and the Logic Analyzer.
- **Alternatives Considered**:
  - *Share PIO0*: Would require pausing CMSIS-DAP and reloading SWD programs on demand, breaking simultaneous debugging while testing I3C targets.
  - *Bitbang on CPU*: Cannot achieve deterministic 12.5 MHz clocking with T-bit parity turnaround and input synchronizer bypass.

### Decision 2: Pin Domain & Dynamic Pin Ownership
- **Choice**: Map I3C to `GPIO 10` (SCL) and `GPIO 11` (SDA) within the Adapter domain (`GPIO 10-19`) using `pin_owner_claim_pins`.
- **Rationale**: Keeps the Adapter domain clean and contiguous. In `pin_owner.c`, GPIO 10 and 11 already support `GPIO_FUNC_PIO2`. Setting base pin to 10 allows consecutive pin indexing required by PIO side-set and in/out pin groups.
- **Alternatives Considered**:
  - *Use Programmer domain (GPIO 0-1)*: Would conflict with target SWD/ISP flashing and violate AgentLA's domain isolation principles.

### Decision 3: Dynamic System Clock Divisor Calculation
- **Choice**: Calculate PIO clock divisor dynamically:
  $$\text{clkdiv} = \frac{\text{clock\_get\_hz(clk\_sys)}}{10 \times f_{\text{target}}}$$
- **Rationale**: In `i3c.pio`, the bit transfer loop takes exactly 10 cycles per bit. Because AgentLA can run at 150 MHz, 200 MHz, or 400 MHz (Turbo), a hardcoded 125 MHz divider table will produce incorrect bus timings. Querying `clock_get_hz(clk_sys)` ensures exact frequency matching.

### Decision 4: Verification Architecture using STM32H503
- **Choice**: Use the STMicroelectronics STM32H503 as the reference target device for hardware-in-the-loop validation.
- **Rationale**:
  - The STM32H503 contains a native MIPI I3C v1.1 peripheral supporting Target mode, ENTDAA, CCC, and IBI.
  - AgentLA already contains built-in SWD flash algorithms and detection for the STM32H503 (`TARGET_PART_STM32H503`, device ID `0x474`).
  - An automated Python test script can flash the STM32H503 target firmware via AgentLA SWD, reset it into target mode, and immediately exercise AgentLA's I3C controller commands against it.
- **Alternatives Considered**:
  - *Software Loopback on RP2350*: Does not test genuine open-drain/push-pull turnaround against a real physical target silicon.

```
┌─────────────────────────────────────────────────────────────┐
│                   Hardware Test Topology                    │
│                                                             │
│   AgentLA Probe (RP2350)              STM32H503 Target      │
│   ┌────────────────────┐              ┌──────────────────┐  │
│   │ SWDCLK   (GPIO 4)  ├─────────────▶│ SWCLK            │  │
│   │ SWDIO    (GPIO 5)  │◀────────────▶│ SWDIO            │  │
│   │ nRST     (GPIO 6)  ├─────────────▶│ NRST             │  │
│   │                    │              │                  │  │
│   │ I3C SCL  (GPIO 10) ├─────────────▶│ I3C1_SCL (PB6)   │  │
│   │ I3C SDA  (GPIO 11) │◀────────────▶│ I3C1_SDA (PB7)   │  │
│   │ GND                ├──────────────┤ GND              │  │
│   └────────────────────┘              └──────────────────┘  │
│                             (4.7kΩ Pull-ups on SCL & SDA)   │
└─────────────────────────────────────────────────────────────┘
```

## Risks / Trade-offs

- **[Risk] High-Speed Signal Integrity at 12.5 MHz** → **Mitigation**: Enable fast slew rate and 12 mA drive strength (`gpio_set_drive_strength`), bypass input synchronizers via `hw_set_bits(&pio2->input_sync_bypass, ...)`, and provide bus error diagnostic counters.
- **[Risk] Target Voltage Mismatch (1.2V/1.8V vs 3.3V)** → **Mitigation**: STM32H503 supports 3.3V VDD, allowing direct 3.3V pull-up connection for verification without level shifters. For low-voltage targets (e.g. 1.0V DDR5 PMIC), document the requirement for the companion bidirectional level-shifter board.
- **[Risk] IBI Arbitration Contention** → **Mitigation**: Implement mandatory ACK/NACK handling in the PIO address phase to ensure AgentLA can either grant or postpone an IBI cleanly without hanging the bus.
