## ADDED Requirements

### Requirement: Modbus RTU Inter-Frame Delimitation and CRC-16 Check
The firmware SHALL detect Modbus RTU packet boundaries using an idle period of at least $3.5 \times t_{\text{char}}$ and validate frame integrity using the Modbus CRC-16 polynomial (`0xA001`, LSB first).

#### Scenario: Valid Read Holding Registers Request
- **WHEN** a Modbus frame with Slave ID 1, Function Code 0x03, Start Address 0x0064, Register Count 2, and valid CRC-16 is captured
- **THEN** the decoder registers a `MODBUS_REQ` event with Slave 1, Function 0x03, Address 100, and Count 2

#### Scenario: Corrupted Modbus frame CRC
- **WHEN** a Modbus packet is received with an invalid CRC-16 checksum
- **THEN** the decoder records the packet with a CRC Error flag and rejects register payload extraction

### Requirement: Standard Function Code and Exception Decoding
The decoder SHALL decode standard Modbus function codes (0x01–0x06, 0x0F, 0x10) and exception responses (Function Code with MSB set).

#### Scenario: Decoding Modbus exception response
- **WHEN** a slave returns Function Code 0x83 with Exception Code 0x02 (Illegal Data Address)
- **THEN** the decoder marks the event as `MODBUS_EXCEPTION` with Illegal Data Address description
