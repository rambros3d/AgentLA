# hw-swd-ack-check Specification

## Purpose
TBD - created by archiving change rewrite-swd-pio-engine. Update Purpose after archive.
## Requirements
### Requirement: PIO validates SWD ACK bits in hardware

The PIO state machine shall check the 3-bit ACK response from the target after each SWD request, without CPU intervention.

#### Scenario: Target responds with ACK OK (0b001)
- **WHEN** an SWD read or write request is sent
- **AND** the target responds with ACK = 0b001 (OK)
- **THEN** the PIO shall push the ACK value to the RX FIFO for the CPU to read
- **AND** no error flag shall be set

#### Scenario: Target responds with ACK FAULT (0b100)
- **WHEN** an SWD read or write request is sent
- **AND** the target responds with ACK = 0b100 (FAULT)
- **THEN** the PIO shall push the ACK value to the RX FIFO
- **AND** the CPU shall receive SWD_ERR_ACK_FAULT

#### Scenario: Target responds with ACK WAIT (0b010)
- **WHEN** an SWD read or write request is sent
- **AND** the target responds with ACK = 0b010 (WAIT)
- **THEN** the PIO shall push the ACK value to the RX FIFO
- **AND** the CPU shall retry up to 16 times before reporting SWD_ERR_ACK_WAIT

#### Scenario: Target does not respond (all zeros or all ones)
- **WHEN** an SWD read or write request is sent
- **AND** the ACK bits are not 0b001, 0b010, or 0b100
- **THEN** the PIO shall push the invalid ACK value to the RX FIFO
- **AND** the CPU shall receive SWD_ERR_PROTOCOL

