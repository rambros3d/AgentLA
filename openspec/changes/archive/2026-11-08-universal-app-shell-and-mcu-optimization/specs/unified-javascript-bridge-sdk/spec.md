## MODIFIED Requirements

### Requirement: Universal AgentLA Client SDK Namespace
The `agentla-bridge.js` script SHALL export a single global `window.AgentLA` object providing typed, promise-based API modules for hardware peripherals, pin crossbar control, telemetry, in-browser MCP execution, and UI utilities. When running embedded inside an iframe (`window.self !== window.top`), the script SHALL suppress the automatic mounting of duplicate `#top-bar` and `#floating-glass-dock` elements, delegating OS chrome management to the parent App-Shell.

#### Scenario: SDK initialization
- **WHEN** any web page or LittleFS app loads `/agentla-bridge.js`
- **THEN** `window.AgentLA` is available on the global object with initialized submodules `system`, `pins`, `can`, `uart`, `i2c`, `spi`, `servo`, `programmer`, `mcp`, `telemetry`, and `ui`.

#### Scenario: Iframe chrome suppression
- **WHEN** `/agentla-bridge.js` executes inside an iframe viewport
- **THEN** the script does not mount duplicate `#top-bar` or `#floating-glass-dock` elements into the child document.
