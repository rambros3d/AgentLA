## ADDED Requirements

### Requirement: Zero-Heap UART Streaming Decoder
The analyzer engine SHALL provide an asynchronous serial (UART) decoder operating without heap allocation, supporting configurable baud rates, 5–9 data bits, parity checking (None, Even, Odd), stop bit validation (1, 1.5, 2), and framing/break error detection.

#### Scenario: Decodes standard 115200 8N1 ASCII sequence
- **WHEN** raw transitions corresponding to byte `0x41` ('A') at 115,200 baud are processed
- **THEN** decoder outputs `DecodedEvent::UartByte { byte: 0x41, parity_err: false, framing_err: false }` with correct start and stop bit alignment

#### Scenario: Detects framing error on missing stop bit
- **WHEN** line remains low during the expected stop-bit interval
- **THEN** decoder outputs `DecodedEvent::UartByte` with `framing_err: true`

### Requirement: Zero-Heap I2C / SMBus Streaming Decoder
The analyzer engine SHALL provide an I2C decoder tracking START, Repeated START, and STOP conditions, decoding 7-bit and 10-bit target addresses, Read/Write indicators, data byte sequences, ACK/NACK status, and SMBus PEC CRC-8 verification.

#### Scenario: Decodes write transaction with ACK
- **WHEN** transitions representing START, address `0x48` with Write bit, slave ACK, and data bytes `[0x01, 0x02]` followed by STOP are received
- **THEN** decoder yields `DecodedEvent::I2cTransaction` containing address `0x48`, `is_read: false`, `ack: true`, and payload bytes `[0x01, 0x02]`

#### Scenario: Flags unacknowledged transaction (NACK)
- **WHEN** slave leaves SDA high during the 9th SCL clock pulse
- **THEN** decoder emits `DecodedEvent::I2cTransaction` with `ack: false`

### Requirement: Zero-Heap SPI Streaming Decoder
The analyzer engine SHALL provide a 4-wire and 3-wire synchronous SPI decoder supporting SPI Modes 0, 1, 2, and 3, active-low and active-high Chip Select framing, and synchronized full-duplex MOSI/MISO byte extraction.

#### Scenario: Decodes Mode 0 full-duplex transfer
- **WHEN** Chip Select is asserted low, 8 clock pulses occur with CPOL=0 and CPHA=0, and CS is deasserted
- **THEN** decoder yields `DecodedEvent::SpiTransfer` with assembled MOSI and MISO bytes and transfer length

### Requirement: Zero-Heap CAN 2.0A / 2.0B Streaming Decoder
The analyzer engine SHALL provide a CAN decoder supporting standard (11-bit) and extended (29-bit) identifiers, Remote Transmission Request (RTR) flags, Data Length Code (DLC), payload unpacking (0–8 bytes), CRC-15 validation, and bit-stuffing error detection.

#### Scenario: Validates CAN 2.0A standard data frame
- **WHEN** a valid CAN frame with ID `0x123`, DLC `4`, payload `[0xDE, 0xAD, 0xBE, 0xEF]`, and matching CRC-15 is ingested
- **THEN** decoder emits `DecodedEvent::CanFrame` with `id: 0x123`, `is_extended: false`, `dlc: 4`, `crc_ok: true`

#### Scenario: Detects bit-stuffing violation
- **WHEN** 6 consecutive identical bit levels are observed prior to the CRC delimiter
- **THEN** decoder flags a bit-stuffing error and emits `DecodedEvent::Error` with CAN bit stuff violation code
