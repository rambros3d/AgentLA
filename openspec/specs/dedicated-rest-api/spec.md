# dedicated-rest-api Specification

## Purpose
Dedicated REST API server providing app lifecycle, filesystem operations, storage telemetry, and standardized JSON error handling.

## Requirements

### Requirement: RESTful App Management Endpoints
The HTTP server SHALL provide REST endpoints under `/api/v1/apps` for listing, inspecting, installing, and deleting applications.

#### Scenario: Listing installed apps via REST
- **WHEN** client sends `GET /api/v1/apps`
- **THEN** server returns HTTP 200 with JSON array of installed app manifests and storage sizes.

#### Scenario: Querying app manifest via REST
- **WHEN** client sends `GET /api/v1/apps/{app-name}/manifest`
- **THEN** server returns HTTP 200 with the `app.json` metadata for the specified app.

### Requirement: Streaming App Installation Endpoint
The HTTP server SHALL accept streaming `.tar` archives and `.html` files at `POST /api/v1/apps/install?name={app-name}` and unpack or write them directly into LittleFS.

#### Scenario: Streaming TAR installation
- **WHEN** client sends `POST /api/v1/apps/install?name=can-sniffer` with a `.tar` payload
- **THEN** the firmware parses tar headers sequentially and writes each file directly to `/apps/can-sniffer/`, returning HTTP 201 Created upon completion.

#### Scenario: Standalone HTML installation
- **WHEN** client sends `POST /api/v1/apps/install?name=test-jig` with an HTML payload (`Content-Type: text/html`)
- **THEN** the firmware saves the content directly as `/apps/test-jig/index.html` and creates a default `app.json`, returning HTTP 201 Created.

### Requirement: Hardware Control and Status Endpoints
The HTTP server SHALL provide clean REST endpoints under `/api/v1/system` and `/api/v1/pins` without requiring JSON-RPC encapsulation.

#### Scenario: Reading system status
- **WHEN** client sends `GET /api/v1/system`
- **THEN** server returns HTTP 200 with board model, MCU flash size, firmware version, and uptime.

#### Scenario: Triggering emergency safe state
- **WHEN** client sends `POST /api/v1/pins/safe`
- **THEN** server quiesces all GPIO pins to high-impedance safe state and returns HTTP 200 with `{ "status": "safe", "pins_quiesced": true }`.
