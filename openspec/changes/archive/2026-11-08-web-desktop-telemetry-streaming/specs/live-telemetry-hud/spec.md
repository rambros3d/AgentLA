## ADDED Requirements

### Requirement: Live System Telemetry Stream and REST Polling
The firmware SHALL expose real-time instrument telemetry via a 10 Hz Server-Sent Events (SSE) push stream at `GET /api/v1/system/telemetry/stream` and via on-demand REST polling at `GET /api/v1/system/telemetry`.

#### Scenario: Real-Time SSE Telemetry Reception
- **WHEN** client opens an EventSource connection to `/api/v1/system/telemetry/stream`
- **THEN** server pushes structured JSON telemetry frames at 10 Hz containing target power, MCU silicon health, memory statistics, and pin ownership states.

#### Scenario: On-Demand REST Telemetry Query
- **WHEN** client performs an HTTP GET request to `/api/v1/system/telemetry`
- **THEN** server returns an immediate JSON response reflecting instantaneous instrument sensor readings.

### Requirement: Target Power and Rail Telemetry
The telemetry engine SHALL sample target rail voltage ($V_{\text{target}}$) and target load current ($I_{\text{target}}$) using the onboard TI INA180 high-side current amplifier model and ADC channels.

#### Scenario: Active Target Current Measurement
- **WHEN** a target MCU draws active current across the INA180 shunt resistor
- **THEN** the firmware converts ADC microvolts to microamps and calculates instantaneous target load current and power in milliwatts.

#### Scenario: Target Voltage Rail Sensing
- **WHEN** a target MCU is connected with $3.3\,\text{V}$ or $1.8\,\text{V}$ supply
- **THEN** the telemetry report reflects the measured rail voltage within $\pm 20\,\text{mV}$ accuracy.

### Requirement: Silicon Health and Memory Diagnostics
The firmware SHALL monitor internal RP2350 junction temperature, $V_{\text{SYS}}$ supply rail, SRAM consumption, and LittleFS storage fill levels.

#### Scenario: Internal Core Junction Temperature
- **WHEN** ADC Channel 4 samples the internal bandgap temperature diode
- **THEN** the firmware computes core temperature in degrees Celsius ($T = 27 - \frac{V - 0.706}{0.001721}$) and includes it in the telemetry payload.

### Requirement: Hardware Overcurrent Protection Alarm
The telemetry engine SHALL support programmable target overcurrent trip thresholds. If target current draw exceeds the configured limit, the system SHALL immediately trip all target I/O pins into safe High-Z state and broadcast an overcurrent trip event.

#### Scenario: Short-Circuit / Overcurrent Trip Event
- **WHEN** $I_{\text{target}}$ exceeds the programmed threshold (e.g. $500\,\text{mA}$)
- **THEN** the Pin Arbiter immediately releases and tristates all target interface pins, turns on an overcurrent visual alarm badge, and reports `"alarm":"overcurrent_trip"` in the telemetry stream.

### Requirement: Floating Desktop HUD Overlay
The Web Desktop UI SHALL render a floating, draggable, and collapsible glassmorphic HUD component displaying live gauges for target power, core temperature, network speed, and an interactive 30-pin color-coded Pin Ownership Matrix.

#### Scenario: Live Pin Matrix Visualization
- **WHEN** a hardware subsystem (e.g., UART, Programmer, or Logic Analyzer) claims pins
- **THEN** the HUD pin bar immediately updates the corresponding GPIO badge colors in real time reflecting active ownership.
