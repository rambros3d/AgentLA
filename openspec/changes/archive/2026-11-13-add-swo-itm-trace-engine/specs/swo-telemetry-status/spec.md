## ADDED Requirements

### Requirement: SWO Receiver Health Telemetry
The system SHALL track running statistics of the SWO receiver, including active state, total bytes received from target, total bytes delivered to host, and total bytes dropped due to overruns.

#### Scenario: Query SWO status via REST
- **WHEN** client issues `GET /api/programmer/swo/status`
- **THEN** system responds with JSON containing `active`, `baud_rate`, `bytes_received`, `bytes_delivered`, and `bytes_dropped`

#### Scenario: Zero dropped bytes during normal streaming
- **WHEN** host polls and consumes trace packets within buffer limits
- **THEN** `bytes_dropped` remains 0 and `bytes_received` matches `bytes_delivered` plus buffered bytes
