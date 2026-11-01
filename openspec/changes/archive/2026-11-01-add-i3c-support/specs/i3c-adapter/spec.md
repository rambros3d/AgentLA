## ADDED Requirements

### Requirement: I3C Controller State Machine and Clock Rate Configuration
The system SHALL initialize an I3C controller engine on RP2350 PIO2 using dedicated Adapter domain pins (GPIO 10 for SCL, GPIO 11 for SDA). The system SHALL compute clock dividers dynamically based on the active system clock (`clock_get_hz(clk_sys)`) to support clock frequencies from 100 kHz up to 12.5 MHz SDR.

#### Scenario: Successful initialization on Adapter pins
- **WHEN** the host issues `I3C_INIT` command with clock rate 12500 kHz
- **THEN** the system claims GPIO 10 and 11 via the pin owner, programs PIO2 with the I3C state machine, configures open-drain and push-pull timing, sets fast slew rate and 12 mA drive strength, and returns `OK I3C INIT freq=12500000 pins=10,11`.

#### Scenario: SCL frequency parameter out of bounds
- **WHEN** the host attempts to configure a clock rate exceeding 12.5 MHz or below 40 kHz
- **THEN** the system rejects the configuration and returns `ERR I3C CLK frequency out of range (40 - 12500 kHz)`.

### Requirement: Dynamic Address Assignment (ENTDAA)
The system SHALL execute the MIPI I3C Dynamic Address Assignment (ENTDAA) protocol sequence using open-drain arbitration to detect targets and assign unique 7-bit dynamic addresses.

#### Scenario: Successful DAA discovery and address assignment
- **WHEN** the host issues `I3C_ENTDAA` with a proposed target address 0x08
- **THEN** the system issues START, 0x7E arbitration header, CCC broadcast ENTDAA (0x07), receives the 48-bit Provisional ID (PID), BCR, and DCR from the target, assigns dynamic address 0x08, validates ACK, and returns `OK I3C ENTDAA addr=0x08 pid=... bcr=... dcr=...`.

#### Scenario: Target NACK or bus collision during DAA
- **WHEN** no responsive I3C target participates in ENTDAA or the target NACKs
- **THEN** the system safely emits a STOP condition and returns `ERR I3C ENTDAA NACK`.

### Requirement: Common Command Codes (CCC) Support
The system SHALL support both Direct CCC and Broadcast CCC transactions in SDR mode conforming to MIPI I3C Basic specification.

#### Scenario: Direct CCC Read
- **WHEN** the host issues `I3C_CCC DIRECT READ <addr> <ccc_id> <len>`
- **THEN** the system transmits the broadcast header 0x7E, the direct CCC byte, repeated START, target address with read bit, collects the requested payload bytes with T-bit validation, and returns the hex-encoded data.

#### Scenario: Broadcast CCC Write
- **WHEN** the host issues `I3C_CCC BC WRITE <ccc_id> <data_hex>`
- **THEN** the system transmits START, 0x7E, the broadcast CCC byte, payload bytes in push-pull SDR mode, emits STOP, and returns `OK I3C CCC BC WRITE`.

### Requirement: Private SDR Read and Write Data Transfers
The system SHALL execute private SDR read, write, and combined write-read transfers to addressed I3C targets up to 12.5 MHz.

#### Scenario: Private write transfer
- **WHEN** the host sends `I3C_WRITE <addr> <hex_data>`
- **THEN** the system transmits START, target dynamic address with W bit, streams the data bytes with T-bit parity generation, terminates with STOP, and returns `OK I3C WRITE <len> bytes`.

#### Scenario: Private read transfer
- **WHEN** the host sends `I3C_READ <addr> <len>`
- **THEN** the system transmits START, target dynamic address with R bit, clocks in the payload with T-bit parity checking, and returns `I3C DATA <hex_data>`.

### Requirement: In-Band Interrupt (IBI) and Hot-Join Monitoring
The system SHALL provide an IBI polling command (`I3C_POLL`) to detect target-initiated In-Band Interrupts or Hot-Join requests during bus idle or address phases.

#### Scenario: Target asserts In-Band Interrupt
- **WHEN** an I3C target pulls SDA low during an arbitration phase or after a START condition
- **THEN** the system clocks the arbitration header, decodes the interrupting target address and mandatory data byte (MDB), acknowledges the request, and returns `OK I3C IBI addr=<addr> mdb=<mdb>`.

#### Scenario: No pending interrupt
- **WHEN** the host polls for IBI and no target requests bus service
- **THEN** the system completes the check without bus disruption and returns `OK I3C NO_IBI`.
