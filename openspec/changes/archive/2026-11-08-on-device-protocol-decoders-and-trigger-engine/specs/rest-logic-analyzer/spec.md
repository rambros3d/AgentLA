## ADDED Requirements

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
