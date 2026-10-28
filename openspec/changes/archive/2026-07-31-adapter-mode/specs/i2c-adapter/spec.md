## ADDED Requirements

### Requirement: I2C1 master adapter
The adapter device SHALL provide I2C master mode using the RP2350 I2C1 hardware peripheral on GPIO0 (SCL) and GPIO1 (SDA).

#### Scenario: I2C read from sensor register
- **WHEN** device receives `I2C_READ addr=0x48 reg=0x00 len=4`
- **THEN** device performs I2C START + address + register pointer read 4 bytes + STOP and returns the bytes

#### Scenario: I2C write to sensor register
- **WHEN** device receives `I2C_WRITE addr=0x48 reg=0x00 data=01,02`
- **THEN** device performs I2C START + address + register write 2 bytes + STOP and responds `OK I2C WRITE 2 bytes`

#### Scenario: I2C NACK from non-existent device
- **WHEN** device receives `I2C_READ addr=0x7F reg=0x00 len=1` (device not present)
- **THEN** device responds with `ERR I2C NACK at addr=0x7F`

#### Scenario: I2C bus stuck (clock stretch timeout)
- **WHEN** a slave holds SCL low for more than 100ms
- **THEN** device times out and responds with `ERR I2C bus stuck`

#### Scenario: I2C bus clear recovery
- **WHEN** bus is stuck, device receives `I2C_CLEAR`
- **THEN** device toggles SCL 9 times and responds `OK I2C BUS CLEARED`

### Requirement: UART1 adapter mode
The adapter device SHALL provide UART master mode using the RP2350 UART1 hardware peripheral on GPIO4 (TX) and GPIO5 (RX).

#### Scenario: UART write at 115200 baud
- **WHEN** device receives `UART_WRITE baud=115200 data=48,65,6C,6C,6F`
- **THEN** device transmits "Hello" (0x48, 0x65, 0x6C, 0x6C, 0x6F) over UART1 TX at 115200 baud and responds `OK UART WROTE 5 bytes`

#### Scenario: UART read with timeout
- **WHEN** device receives `UART_READ baud=9600 len=10 timeout=1000`
- **THEN** device reads up to 10 bytes from UART1 RX with 1000ms timeout and returns `UART DATA 48,65,6C,6C,6F` (or whatever bytes received, possibly empty)

#### Scenario: UART invalid baud rate
- **WHEN** device receives `UART_WRITE baud=100 baud=9600`
- **THEN** device responds with `ERR invalid baud rate, supported: 1200-921600`

#### Scenario: UART RX timeout
- **WHEN** device receives `UART_READ baud=115200 len=10 timeout=100` and no data arrives within 100ms
- **THEN** device responds with `UART DATA` (empty response, no bytes)

### Requirement: IDENTIFY returns adapter role
The adapter CDC device SHALL include its role in the IDENTIFY response.

#### Scenario: Adapter IDENTIFY
- **WHEN** host sends `IDENTIFY` to the adapter CDC port
- **THEN** device responds with `AGENTLA v1.0 RP2350 ADAPTER I2C+UART`

#### Scenario: Analyzer IDENTIFY unchanged
- **WHEN** host sends `IDENTIFY` to the analyzer CDC port
- **THEN** device responds with `AGENTLA v1.0 RP2350 24CH 100MHZ` (no change)

## MODIFIED Requirements

### Requirement: IDENTIFY command
The agent protocol SHALL include a role field (ANALYZER or ADAPTER) in the IDENTIFY response to help the agent distinguish between device types.

#### Scenario: Adapter IDENTIFY includes role
- **WHEN** host sends `IDENTIFY` to the adapter CDC port
- **THEN** device responds with `AGENTLA v1.0 RP2350 ADAPTER I2C+UART`
- **AND** the role field is `ADAPTER`

#### Scenario: Analyzer IDENTIFY includes role
- **WHEN** host sends `IDENTIFY` to the analyzer CDC port
- **THEN** device responds with `AGENTLA v1.0 RP2350 ANALYZER 24CH 100MHZ`
- **AND** the role field is `ANALYZER`