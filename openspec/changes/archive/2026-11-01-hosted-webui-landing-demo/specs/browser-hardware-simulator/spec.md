## ADDED Requirements

### Requirement: Synthetic Multi-Channel Logic Waveforms
The demo simulator SHALL provide realistic pre-generated waveform traces for SPI, I2C, and UART transmissions allowing users to test multi-touch gestures, zoom levels, and channel decoding without connected hardware.

#### Scenario: Running simulated capture
- **WHEN** user clicks "Simulate Capture" or selects a protocol preset in the demo mode
- **THEN** the Logic Analyzer canvas populates with synthetic multi-channel digital transitions and supports full 1-finger horizontal panning and 2-finger pinch-zooming

### Requirement: Dynamic Pin Ownership Playground
The demo simulator SHALL replicate the RP2350 30-pin allocation rules defined in `dynamic-pin-ownership`, strictly enforcing GPIO 0–9 as the fixed programmer block and allowing dynamic driver/observer claims across GPIO 10–29.

#### Scenario: Claiming pins in simulation
- **WHEN** user selects a pin range within GPIO 10–29 and claims a role (e.g. I2C or Logic Observer)
- **THEN** the simulator assigns an active claim handle, updates the 30-pin visual state, and updates simulated driver/observer counts without backend hardware

#### Scenario: Attempting claim on fixed programmer pins
- **WHEN** user attempts to claim GPIO 0–9 for flexible roles in the simulator
- **THEN** the simulator rejects the claim with an explanatory alert that pins 0–9 are permanently reserved for programmer hardware

### Requirement: Mock Live Telemetry Stream
The demo simulator SHALL generate periodic simulated telemetry pulses updating the system clock, core voltage, and probe state indicators.

#### Scenario: Viewing simulated telemetry
- **WHEN** the demo WebUI is open in a browser
- **THEN** the telemetry bar shows 150 MHz sys clock, realistic 1.10 V core voltage reading, and simulated status heartbeat
