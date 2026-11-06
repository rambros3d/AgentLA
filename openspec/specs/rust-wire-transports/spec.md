# rust-wire-transports Specification

## Purpose
PIO hardware microcode wire transports and pin session ownership for ARM SWD, WCH SWIO, and WCH RVSWD debug protocols.

## Requirements

### Requirement: ARM SWD Wire Transport
The firmware SHALL provide a PIO-driven ARM SWD transport (`swd_probe.pio`) capable of generating synchronous clock and bidirectional data sequences up to 18.75 MHz on GPIO 4 and GPIO 5.

#### Scenario: Successful SWD line reset and DPIDR read
- **WHEN** a target SWD session issues a bus reset (50+ clocks high + JTAG-to-SWD sequence `0xE79E`) and reads DP register 0x0
- **THEN** the target returns an `ACK_OK` response and the valid 32-bit `DPIDR` value

#### Scenario: SWD WAIT/FAULT handling
- **WHEN** a target returns `ACK_WAIT` during an access
- **THEN** the transport retries the access up to the configured retry count before returning an error

### Requirement: WCH QingKe 1-Wire SWIO Transport
The firmware SHALL provide a PIO-driven WCH SWIO transport (`swio.pio`) implementing pulse-width modulated single-wire debug on GPIO 5.

#### Scenario: CH32V003 debug module reset and register access
- **WHEN** an 8 µs break pulse is sent and a read is performed on DM register 0x11 (`dmstatus`)
- **THEN** the CH32V003 debug module responds with status indicating core halt and version

### Requirement: WCH QingKe 2-Wire RVSWD Transport
The firmware SHALL provide a PIO-driven WCH RVSWD transport (`rvswd.pio`) implementing 2-wire SDI debug on GPIO 4 and GPIO 5.

#### Scenario: CH32V203 DMI register read
- **WHEN** an RVSWD START condition is pulsed and DMI register 0x10 (`dmcontrol`) is read
- **THEN** the target returns valid 32-bit debug control status with correct parity verification

### Requirement: Exclusive Pin Ownership and Session Switching
The firmware SHALL enforce exclusive hardware pin ownership such that only one active debug session can drive GPIO 4–7 at any time.

#### Scenario: Safe protocol transition
- **WHEN** an ARM SWD session is released and a WCH SWIO session is initialized
- **THEN** the SWD PIO state machine is disabled, GPIOs are safely tristated, and the SWIO engine claims GPIO 5 without bus glitches
