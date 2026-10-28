## ADDED Requirements

### Requirement: IDENTIFY command
The agent protocol SHALL respond to `IDENTIFY` with device model, firmware version, channel count, and maximum sample rate.

#### Scenario: Identify device
- **WHEN** host sends `IDENTIFY\r\n`
- **THEN** device responds with `AGENTLA v1.0 RP2350 24CH 100MHZ\r\n`

#### Scenario: Identify after firmware update
- **WHEN** firmware version changes
- **THEN** IDENTIFY returns the new version number

### Requirement: CONFIGURE command
The agent protocol SHALL accept `CONFIGURE` with channel list, sample rate, trigger, and protocol parameters.

#### Scenario: Configure channels and rate
- **WHEN** host sends `CONFIGURE channels=0,1,2,3 rate=10000000\r\n`
- **THEN** device responds with `OK configured 4 channels at 10000000 Hz\r\n`

#### Scenario: Configure with trigger
- **WHEN** host sends `CONFIGURE channels=0,1 rate=1000000 trigger=RISING pin=0\r\n`
- **THEN** device stores trigger configuration and responds with `OK`

#### Scenario: Invalid parameter
- **WHEN** host sends `CONFIGURE rate=999999999\r\n`
- **THEN** device responds with `ERR invalid sample rate, max 100000000\r\n`

### Requirement: CAPTURE command in RAW mode
The agent protocol SHALL support `CAPTURE mode=RAW` that streams raw pin states to host.

#### Scenario: Raw capture
- **WHEN** host sends `CAPTURE samples=4096 mode=RAW\r\n`
- **THEN** device streams raw hex-encoded pin states and ends with `DONE samples=4096 time=0.0004096\r\n`

#### Scenario: Raw capture with progress
- **WHEN** raw capture is in progress
- **THEN** device sends `CAPTURING...\r\n` before streaming data

### Requirement: CAPTURE command in DECODED mode
The agent protocol SHALL support `CAPTURE mode=DECODED` that performs on-device protocol decode.

#### Scenario: Decoded I2C capture
- **WHEN** host sends `CAPTURE samples=4096 mode=DECODED protocol=I2C sda=0 scl=1\r\n`
- **THEN** device captures, decodes on ARM core, and outputs structured I2C event lines followed by `DONE decoded=<count> transactions=<count>\r\n`

#### Scenario: Decoded capture without protocol
- **WHEN** host sends `CAPTURE samples=4096 mode=DECODED\r\n`
- **THEN** device responds with `ERR protocol required in DECODED mode\r\n`

### Requirement: MEASURE command
The agent protocol SHALL support `MEASURE` for PWM frequency and duty cycle measurement.

#### Scenario: PWM measurement
- **WHEN** host sends `MEASURE PWM pin=0\r\n`
- **THEN** device outputs `PWM FREQ 1000.0 Hz PERIOD 0.001s DUTY 50.0%\r\n`

### Requirement: STATUS command
The agent protocol SHALL support `STATUS` to report current device state.

#### Scenario: Device ready
- **WHEN** host sends `STATUS\r\n`
- **THEN** device responds with `STATUS READY\r\n` when idle, or `STATUS CAPTURING\r\n` when capture is in progress

### Requirement: Line-based text protocol
The agent protocol SHALL use line-based text over USB CDC serial with `\r\n` line endings.

#### Scenario: Command parsing
- **WHEN** host sends `IDENTIFY\r\n`
- **THEN** device reads complete line before processing command

#### Scenario: Unknown command
- **WHEN** host sends `FOOBAR\r\n`
- **THEN** device responds with `ERR unknown command: FOOBAR\r\n`

### Requirement: Human-readable error messages
The agent protocol SHALL return descriptive error messages that LLM agents can interpret and act on.

#### Scenario: Channel out of range
- **WHEN** host sends `CONFIGURE channels=99\r\n`
- **THEN** device responds with `ERR channel 99 out of range, max 23\r\n`

#### Scenario: Rate too high
- **WHEN** host sends `CONFIGURE rate=200000000\r\n`
- **THEN** device responds with `ERR sample rate 200000000 exceeds maximum 100000000\r\n`

### Requirement: Command echo
The agent protocol SHALL echo received commands for debugging.

#### Scenario: Command echo
- **WHEN** host sends `IDENTIFY\r\n`
- **THEN** device echoes `> IDENTIFY\r\n` before sending response
