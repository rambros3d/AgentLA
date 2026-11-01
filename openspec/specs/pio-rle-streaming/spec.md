## ADDED Requirements

### Requirement: Zero-CPU Hardware Run-Length Encoding in PIO
The logic capture system SHALL provide a PIO state machine mode (`pio_rle_capture`) that outputs packed 32-bit `[RunLength:16 | PinState:16]` words directly to DMA only when sampled pin states change or when the 16-bit run counter reaches saturation (65,535 cycles).

#### Scenario: Pin state transitions emit RLE packet
- **WHEN** one or more monitored pins change logic state after $N$ sample ticks ($1 \le N \le 65535$)
- **THEN** the state machine emits a 32-bit packet containing run length $N$ in bits [31:16] and the previous pin state in bits [15:0], resets the counter, and updates the reference state to the new pin state

#### Scenario: Continuous idle lines emit keepalive packet
- **WHEN** no monitored pins change state for 65,535 clock ticks
- **THEN** the state machine emits a 32-bit packet with run length `0xFFFF` and the unchanged pin state, preventing host timeout without overflowing the USB endpoint

### Requirement: Continuous Indefinite Low/Mid-Speed Streaming
The system SHALL support continuous streaming of RLE packets over USB 1.1 bulk endpoints for low- and medium-speed bus traffic (e.g., I2C, UART, CAN-FD, 1-Wire) without buffer overrun or session time limits.

#### Scenario: Indefinite I2C capture over USB 1.1
- **WHEN** capturing an active 400 kHz I2C bus at 50 MS/s sample clock using PIO RLE mode
- **THEN** the average generated USB stream bandwidth remains below 400 kB/s, streaming indefinitely without dropping packets or exhausting internal memory
