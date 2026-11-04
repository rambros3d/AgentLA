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
