## ADDED Requirements

### Requirement: Autonomous Multi-Bus Functional Testing
The test pipeline SHALL support automated release from reset and execution of multi-protocol verification routines including UART boot banner detection, CAN-FD echo, RS-485 ping, and I2C/SPI peripheral query.

#### Scenario: UART boot banner verification
- **WHEN** the target MCU is released from reset
- **THEN** the probe SHALL sniff the target UART TX pin and assert that the expected boot banner string (e.g. `BOOT_OK`) is received within the configured timeout

#### Scenario: Onboard CAN-FD transceiver loopback test
- **WHEN** the recipe includes a `can_transceiver` test step
- **THEN** BakuProbe's onboard TCAN334 SHALL transmit the test CAN frame and verify that the target MCU responds with the expected acknowledgement frame within 200ms

### Requirement: Dynamic Run and Sleep Current Profiling
The test pipeline SHALL measure target current draw during active MCU execution and low-power sleep modes using the INA180 current sense circuit.

#### Scenario: Active power consumption measurement
- **WHEN** the target executes its application firmware
- **THEN** the probe SHALL measure average current draw and assert that it falls between the recipe's minimum and maximum acceptable limits (e.g. 20mA to 60mA)
