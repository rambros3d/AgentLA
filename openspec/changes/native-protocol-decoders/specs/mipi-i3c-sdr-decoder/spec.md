## ADDED Requirements

### Requirement: MIPI I3C SDR Broadcast and Direct CCC Parsing
The firmware SHALL detect the I3C Broadcast Header (`0x7E + W`), identify Common Command Codes (CCC: `ENTDAA`, `SETDASA`, `ENEC`, `DISEC`, `GETPID`), and decode private SDR data transfers with transition `T` parity verification.

#### Scenario: Decoding Dynamic Address Assignment (ENTDAA)
- **WHEN** master issues CCC `0x07` (ENTDAA) followed by 48-bit provisional ID, BCR, DCR, and dynamic address assignment
- **THEN** the decoder logs an `I3C_CCC` event with ENTDAA name and the assigned dynamic address

#### Scenario: Verifying transition bit parity
- **WHEN** an I3C payload byte is transmitted with incorrect `T` parity bit
- **THEN** the decoder flags a `Parity Error` for that byte
