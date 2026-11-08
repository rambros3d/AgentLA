# rest-logic-analyzer Specification

## Purpose
Enables headless acquisition configuration, arming, status monitoring, and zero-RAM VCD streaming for the 10-channel digital logic analyzer over REST.

## Requirements

### Requirement: Logic Analyzer Capture Configuration & Arming
The HTTP server SHALL provide `POST /api/v1/analyzer/arm` accepting sample rate, channel mask (GPIO 20–29), sample depth, and trigger conditions, configuring hardware PIO state machines and DMA channels.

#### Scenario: Arming 4-channel capture
- **WHEN** client sends `POST /api/v1/analyzer/arm` with `{ "channels": [20, 21, 22, 23], "rate": 10000000, "samples": 4096, "trigger_pin": 20, "trigger_edge": "rising" }`
- **THEN** server claims analyzer pins, arms DMA circular buffer, and returns HTTP 200 with `{ "status": "armed", "channels": 4, "rate": 10000000 }`.

### Requirement: Logic Analyzer Capture Status and Control
The HTTP server SHALL provide `GET /api/v1/analyzer/status` reporting capture state (`idle`, `armed`, `capturing`, `done`), samples captured, and trigger timestamp, and `POST /api/v1/analyzer/stop` to force abort an active acquisition.

#### Scenario: Polling capture completion
- **WHEN** client sends `GET /api/v1/analyzer/status` after acquisition completes
- **THEN** server returns HTTP 200 with `{ "state": "done", "samples_captured": 4096, "triggered": true }`.

### Requirement: Zero-RAM VCD Export Streaming
The HTTP server SHALL provide `GET /api/v1/analyzer/vcd` streaming standard Value Change Dump (`.vcd`) ASCII text directly from `captureBuffer` via chunked `fs_read_custom()` transfers with `Content-Type: text/plain`.

#### Scenario: Downloading VCD waveform
- **WHEN** client sends `GET /api/v1/analyzer/vcd`
- **THEN** server streams VCD header and sample transition records chunk-by-chunk without allocating heap memory, terminating with HTTP completion.

### Requirement: Decoder Configuration and Trigger REST Endpoints
The HTTP server SHALL provide REST endpoints to configure active protocol decoders (`POST /api/v1/analyzer/decoders/configure`) and install Hardware Trigger Matrix rules (`POST /api/v1/analyzer/triggers/configure`), validating channel ownership against the pin arbiter.

#### Scenario: Installing trigger rule via REST
- **WHEN** client posts a JSON trigger definition `{ "rule_id": 1, "bus": "i2c", "addr": 72, "ack": false, "actions": ["freeze_capture", "emit_ws_alert"] }`
- **THEN** server populates the trigger matrix table and returns HTTP 200 with `{ "status": "configured", "rule_id": 1 }`

### Requirement: Real-Time Decoded Event WebSocket Streaming
The HTTP server SHALL provide a WebSocket endpoint `GET /api/v1/analyzer/stream/decoded` streaming JSON-serialized `DecodedEvent` objects in real time to connected browser clients.

#### Scenario: Live event streaming over WebSocket
- **WHEN** client opens a WebSocket connection to `/api/v1/analyzer/stream/decoded` while capture is running with active decoders
- **THEN** server pushes text frames containing decoded events with microsecond timestamps and payload fields
