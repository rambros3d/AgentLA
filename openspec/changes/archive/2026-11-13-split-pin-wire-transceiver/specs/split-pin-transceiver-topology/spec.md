## ADDED Requirements

### Requirement: Probe Wire Topology Representation
The system SHALL define a `ProbeWireTopology` model supporting both single-pin bidirectional lines (`Combined { clk, dio }`) and separated driver/sense lines (`Split { clk, dio_drv, dio_sns }`).

#### Scenario: Querying Combined topology pin mapping
- **WHEN** a `Combined { clk: 2, dio: 3 }` topology is instantiated
- **THEN** `dio_out()` returns pin 3, `dio_in()` returns pin 3, and `is_split()` returns false

#### Scenario: Querying Split topology pin mapping
- **WHEN** a `Split { clk: 2, dio_drv: 3, dio_sns: 4 }` topology is instantiated
- **THEN** `dio_out()` returns pin 3, `dio_in()` returns pin 4, and `is_split()` returns true

### Requirement: Zero-Contention Half-Duplex Pin Drive
The system SHALL support driving outbound bits via `dio_out` while simultaneously observing inbound line state on `dio_in`, releasing `dio_out` during reception cycles without delay.

#### Scenario: Turnaround execution in split mode
- **WHEN** the transport finishes transmitting the last host bit
- **THEN** `dio_out` is tri-stated immediately and subsequent bit reads sample `dio_in` without line contention
