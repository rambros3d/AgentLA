## ADDED Requirements

### Requirement: Bounded HTTP Worker Concurrency
The HTTP server running on the RP2350 SHALL operate with a bounded worker pool of at most 2 concurrent listening TCP sockets on port 80 to constrain task future allocations to under 85 KiB of SRAM.

#### Scenario: Concurrent REST requests
- **WHEN** client applications initiate concurrent HTTP requests to `http://10.10.10.10:80`
- **THEN** the firmware SHALL accept and service the connections without exceeding task arena boundaries or triggering watchdog resets.

### Requirement: Non-Crashing Request Processing
The HTTP server SHALL complete TCP connection handshakes and dispatch incoming HTTP requests without triggering HardFaults, panics, or USB bus reset cycles.

#### Scenario: System status request
- **WHEN** a client performs `GET /api/v1/system`
- **THEN** the server returns HTTP 200 with system status JSON and the USB device interface remains continuously active.
