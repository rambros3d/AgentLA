## MODIFIED Requirements

### Requirement: Flash Memory Erase and Program

The HTTP server SHALL publish `POST /api/v1/programmer/erase` and `POST /api/v1/programmer/flash`.

While the build-time flag `HTTP_FLASH_ENABLED` is `false`, both routes SHALL return HTTP
`501 Not Implemented` with a body containing `"error":"http_flash_disabled"`, `"use":"usb"`, and
the supported USB modes. While the flag is `true`, the routes SHALL execute sector or mass erases
and write target flash memory via on-chip RAM algorithm runners.

The routes `POST /api/v1/programmer/reset-catch`, `POST /api/v1/programmer/rescue/stm32`, and
`POST /api/v1/programmer/rescue/nrf` SHALL follow the same gate, because they arm CoreSight vector
catch or perform mass erase.

#### Scenario: HTTP flash disabled

- **WHEN** a client sends `POST /api/v1/programmer/flash` while `HTTP_FLASH_ENABLED` is `false`
- **THEN** the server returns HTTP 501 with
  `{"status":"disabled","error":"http_flash_disabled","use":"usb","modes":["cmsis","stlink"]}`
- **AND** no target flash is modified

#### Scenario: Performing target mass erase

- **WHEN** `HTTP_FLASH_ENABLED` is `true` and the client sends
  `POST /api/v1/programmer/erase` with `{ "mode": "mass" }`
- **THEN** the server unlocks the flash controller, triggers the mass erase, verifies the blank
  state, and returns HTTP 200 with `{ "status": "erased", "mode": "mass" }`

#### Scenario: Non-flash routes remain available

- **WHEN** `HTTP_FLASH_ENABLED` is `false`
- **THEN** `GET /api/v1/programmer/target`, `GET /api/v1/programmer/read`, and
  `POST /api/v1/programmer/reset` continue to return their normal responses

#### Scenario: MCP flash tool reports the same disabled state

- **WHEN** an MCP client calls `programmer_flash` while `HTTP_FLASH_ENABLED` is `false`
- **THEN** the tool returns an error containing `http_flash_disabled` and the supported USB modes
- **AND** no target flash is modified