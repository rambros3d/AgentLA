# logic-analyzer-capture-core Specification

## Purpose
Core high-speed PIO/DMA digital sampling engine, hardware state machine, RLE compression, and REST/SSE control endpoints for the AgentLA Logic Analyzer.

## Requirements

### Requirement: Capture core state machine 
The firmware SHALL expose a capture state machine with states `Idle`, `Armed`, `Capturing`, `Captured`, `Error`, and `Stopped`, along with an API for transitions (`arm`, `start`, `stop`, reporting state).

#### Scenario: Arm command
- **WHEN** client issues `POST /api/v1/logic-analyzer/arm` with no active capture
- **THEN** state changes from Idle to Armed and hardware registers are populated, returning HTTP 200
- **WHEN** called during an active capture
- **THEN** state remains Capturing and HTTP 409 is returned

### Requirement: RLE-compressed in-hardware sampling
The system SHALL stream RLE-compressed words with layout `RunLength:16 bits | PinState:16 bits` through a PIO/DMA ring for at least four simultaneous channels at any supported sample rate up to the documented base width.

#### Scenario: Trigger-free burst
- **WHEN** no-trigger capture is requested for length 1024 at listed pins
- **THEN** sample state returns complete with 2-4 words (RLE proof-of-concept), and 4-state words in buffer

### Requirement: Trigger-qualified capture modes
The system SHALL support rising-edge trigger, falling edge trigger, and immediate/no-trigger mode (Phase 1 subset), and configuration REST endpoints to set trigger pin, pre/post sizes, and pattern. Edge triggers arm PIO `irq 1` to fire post-trigger DMA and stop cleanly when sample completion is signaled.

#### Scenario: Rising edge triggers capture completion
- **WHEN** `POSITIVE_CAPTURE` mode is armed on a pin and a rising edge occurs
- **THEN** sampling then completes and state reaches Captured within the designed post-count window

### Requirement: Capture state exposed via REST and SSE
The firmware SHALL provide `GET /api/v1/logic-analyzer/status`, `POST /api/v1/logic-analyzer/start`, `/stop`, `/config`, and stream telemetry update packet notifications at least once per bank burst via the existing SSE channel.

#### Scenario: Status query
- **WHEN** client queries status before and after run
- **THEN** it receives JSON with current state, `pre_bytes`, `post_bytes`, and available sample length; GET responds with 200

### Requirement: Pin-bank claim via registry
The analyzer SHALL claim one contiguous bank span (channels, pin map but also config trigger) via pin_owner claim before capture and release it on stop, preventing overlap with Servo/Profile-related claims.

#### Scenario: Claim overlaps servo pin
- **WHEN** a capture bank intersects an already-claimed PWM pin
- **THEN** the claim returns HTTP 409 with conflicting pin list and no capture state is initialised

### Requirement: Memory-overflow guard
The static bank allocation SHALL not exceed 416 KB (`SCOPE_TOTAL_SIZE`) bounded at compile-time to keep the firmware below the 520 KB SRAM ceiling; heavy modes (`SCOPE`/`TURBO`) shall fail gracefully (HTTP 503) if the buffer cannot be allocated.

#### Scenario: Out-of-memory
- **WHEN** sampling mode would overrun the 416 KB window limit
- **THEN** the start command is rejected with HTTP 503 and the state remains Idle

### Requirement: Protocol-Qualified Capture Triggering
The logic analyzer capture core SHALL accept trigger qualifications from the Hardware Trigger Matrix, allowing protocol event matches to trip the capture state machine from `Capturing` to `Captured` with configurable post-trigger sample retention.

#### Scenario: Protocol trigger terminates capture
- **WHEN** capture is armed in protocol-trigger mode and the Hardware Trigger Matrix fires
- **THEN** capture engine completes post-trigger sample collection and transitions state to Captured
