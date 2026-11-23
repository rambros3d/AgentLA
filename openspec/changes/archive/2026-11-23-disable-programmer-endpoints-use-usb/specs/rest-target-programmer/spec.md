# Spec Delta

## MODIFIED Requirements

### Requirement: Autonomous Target MCU Identification
The HTTP server SHALL provide `GET /api/v1/programmer/target` which, while programmer endpoints are disabled, returns HTTP 200 with a structured warning directing the client to native USB debug probe endpoints (CMSIS-DAP, ST-Link, or WCH-Link).

#### Scenario: Detecting connected target returns USB redirection warning
- **WHEN** client sends `GET /api/v1/programmer/target`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "programmer_disabled", "message": "Programmer endpoints (API and MCP) are disabled. Connect directly via AgentLA USB programmer interfaces (CMSIS-DAP, ST-Link, or WCH-Link) using probe-rs, pyocd, or openocd.", "use": "usb", "modes": ["cmsis", "stlink", "wch"] }`.

### Requirement: Target Reset and Boot Control
The HTTP server SHALL provide `POST /api/v1/programmer/reset` which, while programmer endpoints are disabled, returns HTTP 200 with a structured warning directing the client to native USB debug probe endpoints or hardware nRESET.

#### Scenario: Calling reset returns USB redirection warning
- **WHEN** client sends `POST /api/v1/programmer/reset`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "programmer_disabled", "message": "Programmer endpoints (API and MCP) are disabled. Connect directly via AgentLA USB programmer interfaces (CMSIS-DAP, ST-Link, or WCH-Link) using probe-rs, pyocd, or openocd.", "use": "usb", "modes": ["cmsis", "stlink", "wch"] }`.

### Requirement: Flash Memory Erase and Program
The HTTP server SHALL provide `POST /api/v1/programmer/erase` and `POST /api/v1/programmer/flash` which, while programmer endpoints are disabled, return HTTP 200 with a structured warning directing the client to native USB debug probe flashing tools.

#### Scenario: Calling flash or erase returns USB redirection warning
- **WHEN** client sends `POST /api/v1/programmer/flash` or `POST /api/v1/programmer/erase`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "programmer_disabled", "message": "Programmer endpoints (API and MCP) are disabled. Connect directly via AgentLA USB programmer interfaces (CMSIS-DAP, ST-Link, or WCH-Link) using probe-rs, pyocd, or openocd.", "use": "usb", "modes": ["cmsis", "stlink", "wch"] }`.

### Requirement: Non-Intrusive Live Memory Read
The HTTP server SHALL provide `GET /api/v1/programmer/read` which, while programmer endpoints are disabled, returns HTTP 200 with a structured warning directing the client to native USB debug probe endpoints.

#### Scenario: Calling read returns USB redirection warning
- **WHEN** client sends `GET /api/v1/programmer/read`
- **THEN** server returns HTTP 200 with `{ "status": "disabled", "warning": "programmer_disabled", "message": "Programmer endpoints (API and MCP) are disabled. Connect directly via AgentLA USB programmer interfaces (CMSIS-DAP, ST-Link, or WCH-Link) using probe-rs, pyocd, or openocd.", "use": "usb", "modes": ["cmsis", "stlink", "wch"] }`.

### Requirement: STM32H5 DBGMCU Low-Power Debug Un-gating
The target programmer SHALL support `DBGMCU_CONFIG` configuration on STM32H5 targets through the direct USB debug probe transports (CMSIS-DAP / ST-Link) rather than HTTP REST or MCP.

#### Scenario: Low-power debug connection via USB probe
- **WHEN** host debug tool connects to STM32H5 via AgentLA USB CMSIS-DAP or ST-Link
- **THEN** target debug clocking is maintained across low-power sleep modes via CoreSight SWD transactions.
