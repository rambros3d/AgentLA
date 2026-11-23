# Spec Delta

## MODIFIED Requirements

### Requirement: I2C Bus Bridge Endpoints
The HTTP server SHALL provide `POST /api/v1/bus/i2c/scan` and `POST /api/v1/bus/i2c/transfer` which, while HTTP bus adapter routes are disabled, return HTTP 200 with a structured warning directing the client to the native `i2c-tiny-usb` USB adapter endpoint.

#### Scenario: Scanning I2C bus returns USB redirection warning
- **WHEN** client sends `POST /api/v1/bus/i2c/scan` or `POST /api/v1/bridge/i2c/scan`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "http_adapters_disabled", "message": "HTTP bus adapter routes are temporarily disabled. Connect directly via AgentLA USB endpoints (i2c-tiny-usb, FTDI MPSSE, AgentLA I3C Bulk, gs_usb CAN, CDC-ACM UART).", "use": "usb", "adapters": ["i2c-tiny-usb", "ftdi-mpsse", "agentla-binary-bulk", "gs_usb", "cdc-acm"] }`.

#### Scenario: Performing I2C transfer returns USB redirection warning
- **WHEN** client sends `POST /api/v1/bus/i2c/transfer` or `POST /api/v1/bridge/i2c/transfer`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "http_adapters_disabled", "message": "HTTP bus adapter routes are temporarily disabled. Connect directly via AgentLA USB endpoints (i2c-tiny-usb, FTDI MPSSE, AgentLA I3C Bulk, gs_usb CAN, CDC-ACM UART).", "use": "usb", "adapters": ["i2c-tiny-usb", "ftdi-mpsse", "agentla-binary-bulk", "gs_usb", "cdc-acm"] }`.

### Requirement: SPI Bus Bridge Transfer Endpoint
The HTTP server SHALL provide `POST /api/v1/bus/spi/transfer` which, while HTTP bus adapter routes are disabled, returns HTTP 200 with a structured warning directing the client to the native `ftdi-mpsse` USB adapter endpoint.

#### Scenario: Performing SPI transfer returns USB redirection warning
- **WHEN** client sends `POST /api/v1/bus/spi/transfer` or `POST /api/v1/bridge/spi/transfer`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "http_adapters_disabled", "message": "HTTP bus adapter routes are temporarily disabled. Connect directly via AgentLA USB endpoints (i2c-tiny-usb, FTDI MPSSE, AgentLA I3C Bulk, gs_usb CAN, CDC-ACM UART).", "use": "usb", "adapters": ["i2c-tiny-usb", "ftdi-mpsse", "agentla-binary-bulk", "gs_usb", "cdc-acm"] }`.

### Requirement: UART and CAN Bus Communication
The HTTP server SHALL provide endpoints under `/api/v1/bus/uart`, `/api/v1/bridge/uart`, `/api/v1/bus/can`, and `/api/v1/bridge/can` which, while HTTP bus adapter routes are disabled, return HTTP 200 with a structured warning directing the client to the native CDC-ACM serial and `gs_usb` CAN USB endpoints.

#### Scenario: Sending UART or CAN payload returns USB redirection warning
- **WHEN** client sends `POST /api/v1/bus/uart/write` or `POST /api/v1/bridge/can/on`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "http_adapters_disabled", "message": "HTTP bus adapter routes are temporarily disabled. Connect directly via AgentLA USB endpoints (i2c-tiny-usb, FTDI MPSSE, AgentLA I3C Bulk, gs_usb CAN, CDC-ACM UART).", "use": "usb", "adapters": ["i2c-tiny-usb", "ftdi-mpsse", "agentla-binary-bulk", "gs_usb", "cdc-acm"] }`.
