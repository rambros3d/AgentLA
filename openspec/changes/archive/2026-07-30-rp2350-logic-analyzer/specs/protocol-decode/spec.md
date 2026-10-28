## ADDED Requirements

### Requirement: I2C decode
The protocol decoder SHALL decode I2C transactions including START, STOP, address, R/W bit, ACK/NACK, and data bytes.

#### Scenario: I2C single byte read
- **WHEN** device receives `DECODE protocol=I2C sda=0 scl=1 samples=4096`
- **THEN** device captures and decodes I2C events outputting structured text lines:
  - `I2C START`
  - `I2C ADDR 0x48 R` (or `W`)
  - `I2C ACK` (or `NACK`)
  - `I2C DATA 0x7F`
  - `I2C STOP`

#### Scenario: I2C multi-byte transfer
- **WHEN** I2C bus transfers 8 bytes from address 0x50
- **THEN** decoder outputs START, ADDR, ACK, 8× DATA with ACK/NACK, STOP in correct sequence

#### Scenario: I2C NACK detection
- **WHEN** master sends NACK after data byte
- **THEN** decoder outputs `I2C NACK` for that byte

#### Scenario: I2C clock stretching
- **WHEN** slave holds SCL low (clock stretching)
- **THEN** decoder waits for SCL release before sampling next bit

### Requirement: SPI decode
The protocol decoder SHALL decode SPI transactions including CS assertion, clock edges, MOSI, and MISO data.

#### Scenario: SPI single byte transfer
- **WHEN** device receives `DECODE protocol=SPI cs=2 sclk=3 mosi=4 miso=5 samples=4096`
- **THEN** device outputs:
  - `SPI CS ASSERT`
  - `SPI MOSI 0xA5`
  - `SPI MISO 0x3C`
  - `SPI CS DEASSERT`

#### Scenario: SPI multi-byte transfer
- **WHEN** CS remains asserted while 16 clock cycles occur
- **THEN** decoder outputs 2 SPI data bytes in sequence

#### Scenario: SPI mode 0 (CPOL=0, CPHA=0)
- **WHEN** clock idle low, data sampled on rising edge
- **THEN** decoder correctly reads bits on rising edges

### Requirement: UART decode
The protocol decoder SHALL decode UART frames including start bit, data bits, optional parity, and stop bit.

#### Scenario: UART 8N1 frame
- **WHEN** device receives `DECODE protocol=UART rx=0 baud=115200 samples=4096`
- **THEN** device outputs:
  - `UART 0x55` (data byte)
  - (repeated for each decoded byte)

#### Scenario: UART with parity error
- **WHEN** received parity bit does not match calculated parity
- **THEN** decoder outputs `UART 0x55 ERR_PARITY`

#### Scenario: UART framing error
- **WHEN** stop bit is not detected (low when expected high)
- **THEN** decoder outputs `UART 0x55 ERR_FRAMING`

#### Scenario: UART break condition
- **WHEN** RX line is held low for more than one frame time
- **THEN** decoder outputs `UART BREAK`

### Requirement: PWM measurement
The protocol decoder SHALL measure PWM signal properties including frequency, duty cycle, and period.

#### Scenario: PWM frequency measurement
- **WHEN** device receives `MEASURE PWM pin=0 samples=4096`
- **THEN** device outputs `PWM FREQ 1000.0 Hz PERIOD 0.001s DUTY 50.0%`

#### Scenario: PWM with variable duty cycle
- **WHEN** signal has 30% duty cycle
- **THEN** device outputs `PWM DUTY 30.0%`

#### Scenario: PWM no signal
- **WHEN** pin has no toggling signal during capture
- **THEN** device outputs `PWM NO_SIGNAL`

### Requirement: Decoded mode on-device processing
The protocol decoder SHALL process captured data on the ARM Cortex-M33 without streaming raw data to host.

#### Scenario: I2C decode without raw data transfer
- **WHEN** device is in DECODED mode
- **THEN** device captures data into SRAM, processes it on ARM core, and outputs only decoded events over USB serial

#### Scenario: Multiple protocols simultaneously
- **WHEN** device receives `DECODE protocol=I2C sda=0 scl=1` and `DECODE protocol=UART rx=2 baud=9600`
- **THEN** device decodes both protocols from the same capture data

### Requirement: Decoded event format
The protocol decoder SHALL output decoded events as human-readable text lines with timestamp, protocol, and payload.

#### Scenario: Event line format
- **WHEN** an I2C START event is decoded at timestamp 0.000123 seconds
- **THEN** device outputs `0.000123 I2C START`

#### Scenario: Data byte format
- **WHEN** an I2C data byte 0x7F is decoded
- **THEN** device outputs `0.000125 I2C DATA 0x7F`

### Requirement: Decode statistics summary
The protocol decoder SHALL output a summary after decode completes.

#### Scenario: I2C decode summary
- **WHEN** capture and decode completes for I2C
- **THEN** device outputs `DONE decoded=<byte_count> transactions=<transaction_count>`
