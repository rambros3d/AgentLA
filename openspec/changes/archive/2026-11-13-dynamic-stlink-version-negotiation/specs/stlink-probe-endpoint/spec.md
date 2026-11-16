## MODIFIED Requirements

### Requirement: ST-Link Version Handshake
The system SHALL respond to the ST-Link `F1` version command reporting the dynamically configured version (default `V2J48M35` / `0x2C23`) to satisfy STM32CubeIDE, STM32CubeProgrammer, and OpenOCD compatibility checks across tool versions.

#### Scenario: Version query command
- **WHEN** the host issues command byte `0xF1` to the debug endpoint
- **THEN** the system returns a 6-byte version response matching the currently configured ST-Link version word
