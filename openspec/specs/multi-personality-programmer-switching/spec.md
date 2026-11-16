# multi-personality-programmer-switching Specification

## Purpose
TBD - created by archiving change add-stlink-wchlink-programmer-modes. Update Purpose after archive.
## Requirements
### Requirement: Always-On Base Instrumentation and Adapters
The system SHALL maintain active USB endpoints for CDC-NCM Ethernet, CDC-ACM UART, SocketCAN `gs_usb`, and `i2c-tiny-usb` in every programmer personality mode.

#### Scenario: Base services active in ST-Link mode
- **WHEN** ST-Link mode is active
- **THEN** CDC-NCM Ethernet, CDC-ACM UART, `gs_usb`, and `i2c-tiny-usb` interfaces remain present and operational

#### Scenario: Base services active in WCH-Link mode
- **WHEN** WCH-Link mode is active
- **THEN** CDC-NCM Ethernet, CDC-ACM UART, `gs_usb`, and `i2c-tiny-usb` interfaces remain present and operational

### Requirement: Dynamic USB Re-enumeration on Personality Change
The system SHALL perform a clean USB bus detachment and re-enumeration with the new personality descriptors when a mode switch is commanded.

#### Scenario: Switch from CMSIS-DAP to ST-Link
- **WHEN** a mode switch to ST-Link is triggered via REST or WebUI
- **THEN** the system disconnects USB D+, reconfigures descriptors to VID `0x0483` and PID `0x3752`, and reconnects within 200 ms

#### Scenario: Switch from ST-Link to WCH-Link
- **WHEN** a mode switch to WCH-Link is triggered via REST or WebUI
- **THEN** the system disconnects USB D+, reconfigures descriptors to VID `0x1a86` and PID `0x8010`, and reconnects within 200 ms

### Requirement: Programmer Personality REST API
The system SHALL provide REST API endpoints to query and switch the active programmer personality.

#### Scenario: Query current personality
- **WHEN** a `GET /api/programmer/mode` request is received
- **THEN** the system returns JSON with the active mode (`cmsis`, `stlink`, or `wchlink`)

#### Scenario: Change active personality
- **WHEN** a `POST /api/programmer/mode` request with `{"mode": "wchlink"}` is received
- **THEN** the system saves the setting to flash, responds with HTTP 200, and triggers re-enumeration

