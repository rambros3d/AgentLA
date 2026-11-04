# rest-bus-bridges Specification

## Purpose
Exposes standard RESTful bus bridge endpoints for I2C, SPI, UART, and CAN hardware peripherals on the Adapter GPIO domain.

## Requirements

### Requirement: I2C Bus Bridge Endpoints
The HTTP server SHALL provide `POST /api/v1/bus/i2c/scan` to detect responding 7-bit devices, and `POST /api/v1/bus/i2c/transfer` to execute atomic write and read transactions on the hardware I2C master.

#### Scenario: Scanning I2C bus
- **WHEN** client sends `POST /api/v1/bus/i2c/scan`
- **THEN** server scans addresses 0x08 to 0x77 and returns HTTP 200 with `{ "status": "ok", "devices": [60, 104] }`.

#### Scenario: Performing I2C register read
- **WHEN** client sends `POST /api/v1/bus/i2c/transfer` with `{ "addr": 104, "write": [117], "read_len": 1 }`
- **THEN** server transmits register pointer 117 (0x75 WHO_AM_I), reads 1 byte, and returns HTTP 200 with `{ "status": "ok", "read": [104] }`.

### Requirement: SPI Bus Bridge Transfer Endpoint
The HTTP server SHALL provide `POST /api/v1/bus/spi/transfer` accepting SPI mode, clock frequency in Hz, and hex/array payload for full-duplex SPI communication.

#### Scenario: Reading SPI Flash JEDEC ID
- **WHEN** client sends `POST /api/v1/bus/spi/transfer` with `{ "mode": 0, "speed_hz": 10000000, "tx_hex": "9F000000", "read_len": 3 }`
- **THEN** server asserts CS, clocks 4 bytes, captures MISO, and returns HTTP 200 with `{ "status": "ok", "rx_hex": "EF4018" }`.

### Requirement: UART and CAN Bus Communication
The HTTP server SHALL provide endpoints under `/api/v1/bus/uart` and `/api/v1/bus/can` for serial string transmission, frame monitoring, and CAN frame injection.

#### Scenario: Transmitting UART string
- **WHEN** client sends `POST /api/v1/bus/uart/write` with `{ "baud": 115200, "data": "AT+VERSION\r\n" }`
- **THEN** server transmits data on adapter UART TX and returns HTTP 200 with `{ "status": "ok", "bytes_sent": 12 }`.
