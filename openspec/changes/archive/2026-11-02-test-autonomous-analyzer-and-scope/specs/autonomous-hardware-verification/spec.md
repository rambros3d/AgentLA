## ADDED Requirements

### Requirement: Autonomous Multi-Signal Target Generation
The system SHALL provide a target test firmware for RP2350 that generates simultaneous reference signals: a digital square wave on GPIO 20, a PWM pulse train on GPIO 21, an analog test waveform / filtered PWM on GPIO 26, periodic UART text transmission on GPIO 12 with echo on GPIO 13, and I2C response on GPIO 10/11.

#### Scenario: Running target test signal generator
- **WHEN** the target Pico 2 is booted with the test generator firmware
- **THEN** it outputs continuous test waveforms and actively responds to UART and I2C adapter queries

### Requirement: Autonomous Logic Analyzer Edge Verification
The system SHALL sample digital pins GPIO 20 and GPIO 21 at configured sample rates (10 kHz to 10 MHz) and verify that observed edge counts, frequency, and duty cycle match target generator parameters within 5% tolerance.

#### Scenario: Verifying digital edge capture
- **WHEN** the host executes `CAPTURE pin=20 count=1000 rate=100000`
- **THEN** the returned raw sample buffer contains periodic alternating states matching the 10 kHz target reference signal

### Requirement: Autonomous Protocol Decoder Verification
The system SHALL capture UART traffic on GPIO 13 and verify using `libdecoders` that decoded ASCII packets match transmitted strings.

#### Scenario: Verifying UART protocol decoding
- **WHEN** the target transmits a test sentence at 115200 baud
- **THEN** the protocol decoder extracts and parses the string without frame or parity errors

### Requirement: Live Full-Duplex Adapter Loopback Verification
The system SHALL verify active master/slave adapter communication by writing data over AgentLA UART0 (GPIO 12) and reading identical echoed data back on UART0 RX (GPIO 13).

#### Scenario: Full-duplex UART echo test
- **WHEN** the host issues `UART_WRITE` with a 32-byte payload
- **THEN** the target echoes the payload back and `UART_READ` returns the identical byte sequence
