# can-bus-analyzer-application Specification

## Purpose
Provides an in-browser CAN 2.0B / CAN-FD packet inspector and interactive automotive bus analyzer installed under `/apps/can-sniffer/`.

## ADDED Requirements

### Requirement: Real-Time CAN Frame Inspector Table
The web application SHALL render incoming CAN frames in a tabular display with timestamp, arbitration ID, frame type (Standard or Extended), Remote Transmission Request (RTR) flag, Data Length Code (DLC), hexadecimal payload bytes, and ASCII representation.

#### Scenario: Live frame reception and display
- **WHEN** incoming CAN frames arrive via WebSocket
- **THEN** rows are appended to the sniffer table displaying microsecond timestamp offset, hex ID, DLC, and space-separated data bytes.

### Requirement: Interactive Frame Filtering
The web application SHALL allow the user to filter displayed frames by hexadecimal arbitration ID or mask, with an auto-scroll pause control.

#### Scenario: Filter by arbitration ID
- **WHEN** user enters a hex ID into the filter input
- **THEN** only frames matching the specified arbitration ID or pattern remain visible in the viewport.

### Requirement: Manual and Periodic Frame Transmission
The web application SHALL allow the user to construct and inject standard or extended CAN frames with arbitrary ID, DLC, and payload bytes, supporting both single-shot transmission and periodic interval repeating.

#### Scenario: Inject single CAN frame
- **WHEN** user specifies ID `0x7DF` with DLC `8` and payload `02 01 0D 55 55 55 55 55` and clicks "Send Frame"
- **THEN** the application transmits the frame payload across the WebSocket bridge to the hardware transceiver.

### Requirement: Physical Layer and Bus Controls
The web application SHALL provide controls to activate/deactivate the CAN bridge, set the baud rate (10k, 20k, 50k, 100k, 125k, 250k, 500k, 800k, 1000k), and toggle the switchable $120\,\Omega$ termination resistor on GPIO 22.

#### Scenario: Toggle 120 Ohm termination
- **WHEN** user toggles the $120\,\Omega$ termination switch
- **THEN** the application sends a `POST /api/v1/bridge/can/termination` request with the updated boolean state and updates the UI status badge.
