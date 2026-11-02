## ADDED Requirements

### Requirement: CAN 2.0B and CAN-FD Bitstream Decoding
The firmware SHALL decode raw digital CAN transmissions, synchronizing on Start of Frame (SOF) with a configurable nominal and data-phase sample point (default $75\%-80\%$), removing bit-stuffing bits, and validating CRC integrity.

#### Scenario: Decoding standard 11-bit Classic CAN frame
- **WHEN** a 500 kbps CAN 2.0A frame with standard 11-bit ID `0x123`, DLC 4, and payload `[0xDE, 0xAD, 0xBE, 0xEF]` is captured
- **THEN** the decoder extracts ID `0x123`, verifies the 15-bit CRC, and registers an event with the matching 4 data bytes and CRC OK status

#### Scenario: Decoding 29-bit Extended CAN-FD frame with Bit Rate Switch
- **WHEN** a CAN-FD frame with 29-bit extended ID `0x18DAF110`, BRS enabled, DLC 16, and data-phase rate of 2 Mbps is captured
- **THEN** the decoder transitions to data-phase bit timing, extracts 16 bytes, verifies the 17-bit CRC, and reports the frame with BRS active

### Requirement: Bit-Stuffing and Framing Error Detection
The decoder SHALL monitor the bitstream for 6 consecutive identical bits during stuffed bitstream regions and flag a `Stuff Error` when detected.

#### Scenario: Detecting bit-stuffing violation
- **WHEN** 6 consecutive recessive or dominant bits occur between SOF and CRC Delimiter
- **THEN** the decoder logs a `Stuff Error` event with the timestamp of the 6th invalid bit
