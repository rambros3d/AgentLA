# unified-javascript-bridge-sdk Specification

## Purpose
TBD - created by archiving change unified-mcp-and-javascript-bridge-sdk. Update Purpose after archive.
## Requirements
### Requirement: Universal AgentLA Client SDK Namespace
The `agentla-bridge.js` script SHALL export a single global `window.AgentLA` object providing typed, promise-based API modules for hardware peripherals, pin crossbar control, telemetry, in-browser MCP execution, and UI utilities. When running embedded inside an iframe (`window.self !== window.top`), the script SHALL suppress the automatic mounting of duplicate `#top-bar` and `#floating-glass-dock` elements, delegating OS chrome management to the parent App-Shell.

#### Scenario: SDK initialization
- **WHEN** any web page or LittleFS app loads `/agentla-bridge.js`
- **THEN** `window.AgentLA` is available on the global object with initialized submodules `system`, `pins`, `can`, `uart`, `i2c`, `spi`, `servo`, `programmer`, `mcp`, `telemetry`, and `ui`.

#### Scenario: Iframe chrome suppression
- **WHEN** `/agentla-bridge.js` executes inside an iframe viewport
- **THEN** the script does not mount duplicate `#top-bar` or `#floating-glass-dock` elements into the child document.

### Requirement: Resilient CAN Bus WebSocket Streaming
The `AgentLA.can` module SHALL manage the `/api/v1/can/ws` WebSocket connection, automatically re-establishing disconnected sockets and dispatching parsed frame events.

#### Scenario: Open CAN stream and receive frames
- **WHEN** an application calls `AgentLA.can.openStream({ onFrame, onError, onStatus })`
- **THEN** the SDK connects to the CAN WebSocket, translates incoming messages to `{ id, data, dlc, extended, rtr }` objects, and calls `onFrame` for each packet.

#### Scenario: Automatic stream reconnection
- **WHEN** the physical connection or probe resets and the WebSocket closes unexpectedly
- **THEN** the SDK emits status `reconnecting` and attempts to reconnect with exponential backoff without requiring app reloading.

### Requirement: Automatic Safe-State Teardown on Unload
The SDK SHALL register `beforeunload` and `pagehide` event listeners to release all GPIO pins claimed during the browser session and enter safe state.

#### Scenario: Page navigation or tab close
- **WHEN** the user navigates away from or closes a tab that has claimed hardware pins
- **THEN** the SDK immediately dispatches a keepalive beacon or synchronous release request to return claimed pins to High-Z safe state.

### Requirement: Offline Virtual Hardware Simulation (Demo Parity)
When running in demo mode (GitHub Pages or local file context without hardware), the SDK SHALL automatically route API requests to an in-memory virtual hardware simulator rather than failing network requests.

#### Scenario: Demo mode CAN transmission
- **WHEN** `AgentLA.isDemo` is `true` and the app calls `AgentLA.can.send(...)`
- **THEN** the SDK simulates bus echo and periodic virtual target telemetry without raising network errors.

