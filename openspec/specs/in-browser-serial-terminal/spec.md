# in-browser-serial-terminal

## Purpose
Specifies the in-browser serial terminal (`/apps/terminal`) operating over native WebSockets (`/api/v1/terminal/ws`), providing full-duplex bidirectional target serial communication, hybrid hardware/PIO UART routing, 2 KiB rolling SRAM circular backlog, and hardware target reset control.

## Requirements

### Requirement: Full-Duplex WebSocket Serial Streaming
The firmware SHALL provide a WebSocket endpoint at `GET /api/v1/terminal/ws` upgraded via standard HTTP handshake, streaming bidirectional UART character data between the connected browser and target hardware serial pins.

#### Scenario: Successful WebSocket Upgrade
- **WHEN** client issues an HTTP GET to `/api/v1/terminal/ws` with `Upgrade: websocket`
- **THEN** server responds with `101 Switching Protocols` and establishes a full-duplex WebSocket session.

#### Scenario: Bi-Directional Character Echo
- **WHEN** client sends a WebSocket text or binary frame containing user keystrokes
- **THEN** server immediately transmits the bytes onto the target UART TX pin, and any characters received on the target UART RX pin are forwarded back as WebSocket frames with sub-millisecond latency.

### Requirement: Hybrid UART Hardware and PIO Driver
The firmware SHALL use native hardware `UART1` when default fixed pins (GPIO 8 TX, GPIO 9 RX) are requested, and SHALL fall back automatically to an asynchronous PIO state machine UART when flexible pool pins are specified.

#### Scenario: Default Fixed Pins Mapping
- **WHEN** client connects without explicit pin overrides or specifies `tx=8&rx=9`
- **THEN** server claims pins through the Pin Arbiter and routes transmission through hardware `UART1`.

#### Scenario: Flexible Pool Pins Mapping
- **WHEN** client specifies flexible pool pins (e.g. `tx=14&rx=15`)
- **THEN** server claims the requested pins and activates a PIO UART driver for transmission and reception.

#### Scenario: Pin Conflict Rejection
- **WHEN** client requests pins currently owned by another active driver (e.g., SWD programmer)
- **THEN** server rejects the WebSocket connection before upgrade with HTTP 409 Conflict and error payload `{"error":"pins_busy"}`.

### Requirement: Rolling SRAM Circular Backlog
The firmware SHALL maintain a 2 KiB circular ring buffer in SRAM for UART RX data, buffering incoming target serial bytes even when no browser client is actively connected.

#### Scenario: Preserving Boot Banners
- **WHEN** a target MCU boots and outputs initialization logs while the terminal window is closed
- **THEN** upon client WebSocket connection, the server immediately drains and transmits the buffered 2 KiB backlog before streaming live serial data.

### Requirement: Web Terminal Application UI and Hardware Reset
The firmware SHALL serve an in-browser serial terminal application at `/apps/terminal/index.html` hosting an xterm.js instance with baud rate selection, pin configuration, local echo toggle, log download, and a 1-click hardware target reset button.

#### Scenario: One-Click Target Cold Boot
- **WHEN** user clicks the "Reset Target" button in the terminal UI
- **THEN** the firmware asserts a 50 ms active-low pulse on `nRESET` (GPIO 6), rebooting the target MCU while keeping the terminal session open to capture the startup output.
