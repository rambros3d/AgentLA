# swo-hardware-receiver Specification

## Purpose
TBD - created by archiving change add-swo-itm-trace-engine. Update Purpose after archive.
## Requirements
### Requirement: SWO Hardware Capture Engine
The system SHALL capture Serial Wire Output (SWO) NRZ/UART asynchronous serial data on the configured hardware pin at baud rates from 9600 baud up to 4.5 Mbaud.

#### Scenario: Start SWO capture
- **WHEN** SWO capture is started with a valid baud rate (e.g. 2,000,000 baud)
- **THEN** the hardware UART/PIO receiver is initialized and incoming trace bytes are pushed to the internal ring buffer

#### Scenario: Stop SWO capture
- **WHEN** SWO capture is stopped
- **THEN** the hardware receiver is disabled and the active capture state is marked inactive

### Requirement: SWO Buffer Overrun Protection
The system SHALL manage ring buffer capacity and discard the oldest half of the buffer when unread data exceeds the safety threshold (`SWO_SIZE - 256`), recording the count of discarded bytes.

#### Scenario: Buffer overrun recovery
- **WHEN** unconsumed bytes exceed capacity threshold due to host polling pause
- **THEN** the system advances the consume pointer by half the buffer size and increments the dropped byte counter without stalling

