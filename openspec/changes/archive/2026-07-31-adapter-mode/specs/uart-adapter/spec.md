## ADDED Requirements

### Requirement: UART1 master adapter mode
The adapter device SHALL provide UART master mode using the RP2350 UART1 hardware peripheral on GPIO4 (TX) and GPIO5 (RX).

#### Scenario: UART write at 115200 baud
- **WHEN** device receives `UART_WRITE baud=115200 data=48,65,6C,6C,6F`
- **THEN** device transmits "Hello" over UART1 TX at 115200 baud and responds `OK UART WROTE 5 bytes`

#### Scenario: UART read with timeout
- **WHEN** device receives `UART_READ baud=9600 len=10 timeout=1000`
- **THEN** device reads up to 10 bytes from UART1 RX with 1000ms timeout and returns `UART DATA 48,65,6C,6C,6F` (or whatever bytes received, possibly empty)

#### Scenario: UART invalid baud rate
- **WHEN** device receives `UART_WRITE baud=100`
- **THEN** device responds with `ERR invalid baud rate, supported: 1200-921600`

#### Scenario: UART RX timeout (no data)
- **WHEN** device receives `UART_READ baud=115200 len=10 timeout=100` and no data arrives within 100ms
- **THEN** device responds with `UART DATA` (empty response, zero bytes)

#### Scenario: UART parity and framing errors
- **WHEN** UART receiver detects a framing error during UART_READ
- **THEN** device includes `ERR_FRAMING` in the UART DATA response for that byte

### Requirement: UART adapter baud rate support
The UART adapter SHALL support standard baud rates from 1200 to 921600 using the UART1 hardware peripheral.

#### Scenario: Supported standard baud rates
- **WHEN** device receives `UART_WRITE baud=115200`
- **THEN** device configures UART1 at 115200 baud successfully

#### Scenario: Baud rate out of range
- **WHEN** device receives `UART_WRITE baud=100`
- **THEN** device responds with `ERR invalid baud rate, min=1200 max=921600`