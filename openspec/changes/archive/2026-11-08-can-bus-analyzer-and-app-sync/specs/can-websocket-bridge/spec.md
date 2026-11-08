# can-websocket-bridge Specification

## Purpose
Provides a low-latency, full-duplex WebSocket streaming endpoint for bidirectional CAN bus communication between browser web applications and AgentLA firmware.

## ADDED Requirements

### Requirement: CAN WebSocket Connection Endpoint
The HTTP server SHALL provide a WebSocket route at `/api/v1/can/ws` that accepts connections and negotiates full-duplex text or binary messaging for CAN frames.

#### Scenario: Successful WebSocket handshake
- **WHEN** a client initiates an HTTP Upgrade request to `/api/v1/can/ws`
- **THEN** the server accepts the upgrade with HTTP 101 Switching Protocols and enters the streaming session.

### Requirement: Bidirectional JSON Frame Streaming
The WebSocket bridge SHALL broadcast received CAN frames from the hardware transceiver as JSON objects to connected clients, and SHALL transmit outbound CAN frames onto the physical bus when valid frame objects are received from the client.

#### Scenario: Forwarding incoming bus frame
- **WHEN** the physical CAN transceiver receives a frame on the bus
- **THEN** the bridge formats the frame into a JSON payload with `id`, `ext`, `rtr`, `dlc`, `data`, and `ts` fields and emits it to connected WebSocket clients.

#### Scenario: Injecting outbound frame from client
- **WHEN** client sends a JSON text frame specifying arbitration ID and payload bytes
- **THEN** the bridge validates the frame syntax and queues it for transmission onto the CAN bus via the hardware transceiver.
