## ADDED Requirements

### Requirement: 1-Wire Dallas Reset/Presence and ROM Decoding
The firmware SHALL measure Reset pulse LOW duration ($480-960\,\mu\text{s}$) and Presence pulse ($60-240\,\mu\text{s}$), extract 64-bit ROM registration numbers, and verify the 8-bit DOW CRC ($x^8 + x^5 + x^4 + 1$).

#### Scenario: Reading 64-bit ROM ID
- **WHEN** a 1-Wire transaction executes command `0x33` (Read ROM) returning Family Code `0x28` and serial number `0x00000A123456` with valid DOW CRC
- **THEN** the decoder registers a `1WIRE_ROM` event with Family 0x28 (DS18B20), the 48-bit serial number, and CRC OK status

### Requirement: DS18B20 Temperature Auto-Extraction
The decoder SHALL recognize command `0xBE` (Read Scratchpad) from family `0x28` devices and automatically calculate the temperature in degrees Celsius from scratchpad bytes 0 and 1.

#### Scenario: Calculating DS18B20 temperature
- **WHEN** Read Scratchpad returns bytes `0xAA 0x01` (raw 0x01AA = 426)
- **THEN** the decoder outputs converted temperature `26.62°C`
