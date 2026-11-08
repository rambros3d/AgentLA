## ADDED Requirements

### Requirement: Dedicated LED WebSocket Endpoint
The AgentLA firmware SHALL expose a WebSocket endpoint at `/api/v1/led/ws` that accepts client connections, parses protocol configuration handshakes, and streams high-throughput binary frame payloads directly to addressable LED hardware peripherals.

#### Scenario: Client WebSocket handshake
- **WHEN** a client initiates a WebSocket connection to `ws://10.10.10.10/api/v1/led/ws`
- **THEN** the firmware completes the HTTP 101 Switching Protocols upgrade and awaits configuration or frame messages

### Requirement: Dynamic Pin Acquisition via Pin Arbiter
The LED WebSocket handler SHALL parse requested GPIO data and clock pins from client messages, claim them from `pin_owner` under driver `Ws2812` or `Adapter`, and release them when the socket closes or when Safe State is triggered.

#### Scenario: Successful pin claim
- **WHEN** client sends a config message requesting Data on `GPIO 22`
- **THEN** firmware claims `GPIO 22` via `pin_owner::claim` and returns an acknowledgment `{"status":"ok","pin":22}`

#### Scenario: Pin conflict handling
- **WHEN** client requests a pin currently claimed by another driver
- **THEN** firmware returns an error message `{"status":"error","message":"pin conflict"}` and preserves existing claims

### Requirement: Binary Frame Output Dispatch
Upon receiving binary WebSocket payloads, the firmware SHALL transmit the byte array using the configured protocol driver (PIO for 1-wire, SPI for 2-wire) and return a lightweight acknowledgment to prevent transmission backpressure.

#### Scenario: Transmitting WS2812 binary frame
- **WHEN** client sends a binary frame containing 180 bytes (60 GRB pixels)
- **THEN** the firmware streams the bits out of the data pin with cycle-accurate 800 kHz timing and sends a 1-byte ACK back to the client
