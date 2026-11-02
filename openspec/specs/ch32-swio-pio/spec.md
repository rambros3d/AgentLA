# ch32-swio-pio Specification

## Purpose
Specifies the RP2350 PIO state machine implementation of the single-wire debug interface (SWIO) for CH32V003 microcontrollers.

## Requirements

### Requirement: PIO-based SWIO protocol engine
The system SHALL implement a PIO-based single-wire debug interface (SWIO) for WCH CH32V003 microcontrollers, replacing the current GPIO bit-bang implementation.

#### Scenario: SWIO PIO initialization
- **WHEN** the user issues `CH32 DETECT mode=1wire` or auto-detect selects SWIO
- **THEN** the system SHALL initialize PIO1 SM1 with the SWIO PIO program
- **AND** the PIO clock SHALL be configured to 125 MHz / 12 (~10.4 MHz) for normal mode timing

#### Scenario: SWIO register write
- **WHEN** the system needs to write a 32-bit value to a debug register via SWIO
- **THEN** the PIO SHALL transmit: start bit, 7-bit address (inverted), R/W=1, parity, turnaround, 32-bit data, parity, trailer
- **AND** the operation SHALL complete within 50µs

#### Scenario: SWIO register read
- **WHEN** the system needs to read a 32-bit value from a debug register via SWIO
- **THEN** the PIO SHALL transmit: start bit, 7-bit address (inverted), R/W=0, parity, turnaround
- **AND** then sample 32 data bits + 1 parity bit from the target
- **AND** the operation SHALL complete within 50µs

#### Scenario: SWIO reset sequence
- **WHEN** the system needs to reset the SWIO debug interface
- **THEN** the system SHALL drive the SWIO pin LOW for >8µs
- **AND** then release to HIGH and re-initialize the PIO program

### Requirement: SWIO PIO program fits in PIO1 instruction memory
The SWIO PIO program SHALL use no more than 32 instructions to fit within a single PIO block's instruction memory alongside the existing SWD program.

#### Scenario: PIO program size verification
- **WHEN** the build system compiles the SWIO PIO program
- **THEN** the generated program SHALL be ≤ 32 instructions
- **AND** it SHALL coexist with the SWD program on PIO1 (different SM)

### Requirement: SWIO coexists with SWD on PIO1
The SWIO engine (SM1) and SWD engine (SM0) SHALL share PIO1 without conflict.

#### Scenario: Switching from SWD to SWIO
- **WHEN** the system switches from SWD mode to SWIO mode
- **THEN** it SHALL deinitialize the SWD PIO state machine
- **AND** initialize the SWIO PIO state machine on SM1
- **AND** reconfigure GPIO pin functions as needed

#### Scenario: Switching from SWIO to SWD
- **WHEN** the system switches from SWIO mode to SWD mode
- **THEN** it SHALL deinitialize the SWIO PIO state machine
- **AND** reinitialize the SWD PIO state machine on SM0
