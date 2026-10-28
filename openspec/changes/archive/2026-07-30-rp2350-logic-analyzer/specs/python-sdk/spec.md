## ADDED Requirements

### Requirement: Device connection
The Python SDK SHALL provide a `connect()` function that establishes a serial connection to the AgentLA device.

#### Scenario: Connect to device
- **WHEN** user calls `agentla.connect("/dev/ttyACM0")`
- **THEN** SDK opens serial connection and returns a Device object

#### Scenario: Auto-detect device
- **WHEN** user calls `agentla.connect()` without port
- **THEN** SDK scans available serial ports and connects to AgentLA device if found

#### Scenario: Device not found
- **WHEN** no AgentLA device is found on any serial port
- **THEN** SDK raises `DeviceNotFoundError`

### Requirement: Device identification
The Python SDK SHALL provide `device.identify()` that returns device capabilities.

#### Scenario: Identify device
- **WHEN** user calls `device.identify()`
- **THEN** SDK returns `DeviceInfo(model="AGENTLA", version="v1.0", mcu="RP2350", channels=32, max_sample_rate=100_000_000)`

### Requirement: Channel configuration
The Python SDK SHALL provide `device.configure()` with named parameters for channels, sample rate, and trigger.

#### Scenario: Configure 4 channels at 10 MHz
- **WHEN** user calls `device.configure(channels=[0,1,2,3], sample_rate=10_000_000)`
- **THEN** SDK sends `CONFIGURE channels=0,1,2,3 rate=10000000` and returns True

#### Scenario: Configure with trigger
- **WHEN** user calls `device.configure(channels=[0,1], sample_rate=1_000_000, trigger="RISING", trigger_pin=0)`
- **THEN** SDK sends `CONFIGURE channels=0,1 rate=1000000 trigger=RISING pin=0`

### Requirement: Raw capture
The Python SDK SHALL provide `device.capture_raw()` that returns raw sample data.

#### Scenario: Capture raw data
- **WHEN** user calls `device.capture_raw(samples=4096)`
- **THEN** SDK sends `CAPTURE samples=4096 mode=RAW`, receives hex data, and returns a `RawCapture` object with sample data

#### Scenario: Raw capture with progress callback
- **WHEN** user calls `device.capture_raw(samples=4096, progress_callback=on_progress)`
- **THEN** SDK calls `on_progress` with progress updates during capture

### Requirement: Decoded capture
The Python SDK SHALL provide `device.capture_decoded()` that returns structured protocol events.

#### Scenario: Capture I2C decoded
- **WHEN** user calls `device.capture_decoded(protocol="I2C", sda=0, scl=1, samples=4096)`
- **THEN** SDK returns a list of `I2CEvent` objects with type, timestamp, and data fields

#### Scenario: Capture SPI decoded
- **WHEN** user calls `device.capture_decoded(protocol="SPI", cs=2, sclk=3, mosi=4, miso=5, samples=4096)`
- **THEN** SDK returns a list of `SPIEvent` objects

#### Scenario: Capture UART decoded
- **WHEN** user calls `device.capture_decoded(protocol="UART", rx=0, baud=115200, samples=4096)`
- **THEN** SDK returns a list of `UARTEvent` objects with byte values and error flags

### Requirement: PWM measurement
The Python SDK SHALL provide `device.measure_pwm()` that returns frequency, period, and duty cycle.

#### Scenario: Measure PWM
- **WHEN** user calls `device.measure_pwm(pin=0)`
- **THEN** SDK returns a `PWMResult` object with `frequency`, `period`, and `duty_cycle` fields

### Requirement: Device status
The Python SDK SHALL provide `device.status()` that returns current device state.

#### Scenario: Check status
- **WHEN** user calls `device.status()`
- **THEN** SDK returns `Status.IDLE` or `Status.CAPTURING`

### Requirement: Context manager support
The Python SDK SHALL support `with` statement for automatic connection cleanup.

#### Scenario: Context manager
- **WHEN** user writes `with agentla.connect() as device:`
- **THEN** SDK automatically closes serial connection on exit

### Requirement: Event data classes
The Python SDK SHALL define typed event classes for each protocol.

#### Scenario: I2C event fields
- **WHEN** I2C decode produces an event
- **THEN** `I2CEvent` has fields: `timestamp` (float), `type` (str: START/STOP/ADDR/DATA/ACK/NACK), `value` (optional int for DATA/ADDR), `rw` (optional str: R/W for ADDR)

#### Scenario: SPI event fields
- **WHEN** SPI decode produces an event
- **THEN** `SPIEvent` has fields: `timestamp` (float), `type` (str: CS_ASSERT/CS_DEASSERT/MOSI/MISO), `value` (int for MOSI/MISO)

#### Scenario: UART event fields
- **WHEN** UART decode produces an event
- **THEN** `UARTEvent` has fields: `timestamp` (float), `byte` (int), `error` (optional str: ERR_PARITY/ERR_FRAMING)

### Requirement: Export to Sigrok
The Python SDK SHALL support exporting raw capture data in Sigrok-compatible format.

#### Scenario: Export to Sigrok session
- **WHEN** user calls `device.capture_raw(samples=4096)` then `capture.export_sigrok("session.sr")`
- **THEN** SDK writes a Sigrok session file that can be opened in PulseView
