## ADDED Requirements

### Requirement: CAN Bus MCP Tools
The MCP server SHALL expose `can_status`, `can_send`, `can_sniff`, and `can_set_termination` in `tools/list` and execute them via `tools/call`.

#### Scenario: Query CAN bus status
- **WHEN** client invokes `tools/call` with name `can_status`
- **THEN** server returns JSON with controller state, active bitrate, error counters, and termination relay status.

#### Scenario: Transmit CAN frame
- **WHEN** client invokes `tools/call` with name `can_send` containing `{ id, data_hex, extended }`
- **THEN** firmware validates payload length (up to 8 bytes), transmits the frame onto the physical bus, and returns success JSON.

#### Scenario: Bounded CAN sniffing
- **WHEN** client invokes `tools/call` with name `can_sniff` specifying duration and maximum frame count
- **THEN** firmware captures up to 16 frames from the RX buffer within the time budget and returns an array of decoded frames in JSON.

#### Scenario: Toggle 120Ω bus termination
- **WHEN** client invokes `tools/call` with name `can_set_termination` with `{"enabled": true}`
- **THEN** firmware engages the physical 120Ω termination circuit and returns updated status.

### Requirement: Full-Duplex SPI Master MCP Tool
The MCP server SHALL expose `spi_transfer` allowing AI agents to exchange bytes with SPI peripheral chips.

#### Scenario: Execute SPI byte exchange
- **WHEN** client invokes `tools/call` with name `spi_transfer` providing `mosi_hex` and chip-select configuration
- **THEN** firmware clocks the data over hardware SPI/PIO and returns `miso_hex` in the response envelope.

### Requirement: Serial UART Command MCP Tool
The MCP server SHALL expose `uart_command` allowing AI agents to interact with target serial consoles and bootloaders.

#### Scenario: Send serial command and await response
- **WHEN** client invokes `tools/call` with name `uart_command` providing `tx_data`, `baud`, and `timeout_ms`
- **THEN** firmware transmits data over target UART, captures response bytes until timeout or delimiter, and returns received text.
