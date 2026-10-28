## ADDED Requirements

### Requirement: SWD Engine Initialization
The SWD engine SHALL initialize PIO1 state machine for SWD communication on GPIO 6 (SWCLK) and GPIO 7 (SWDIO).

#### Scenario: Successful initialization
- **WHEN** swd_init() is called
- **THEN** PIO1 state machine is configured with SWCLK as sideset pin and SWDIO as bidirectional pin

#### Scenario: Pin conflict detection
- **WHEN** swd_init() is called while PIO0 is active
- **THEN** initialization SHALL fail with appropriate error code

### Requirement: SWD Write Bits
The SWD engine SHALL write 1-256 bits to SWDIO with SWCLK generation.

#### Scenario: Write 8-bit request
- **WHEN** probe_write_bits(8, 0x7F) is called
- **THEN** 8 bits are clocked out on SWDIO with SWCLK toggling

#### Scenario: Write with turnaround
- **WHEN** probe_write_bits is called after a read operation
- **THEN** SWDIO direction SHALL change with 1-cycle turnaround

### Requirement: SWD Read Bits
The SWD engine SHALL read 1-256 bits from SWDIO.

#### Scenario: Read 3-bit ACK
- **WHEN** probe_read_bits(3) is called after request phase
- **THEN** 3 bits are sampled from SWDIO on SWCLK rising edge

#### Scenario: Read 32-bit data
- **WHEN** probe_read_bits(32) is called during data phase
- **THEN** 32 bits are read LSB-first from SWDIO

### Requirement: SWD Clock Frequency
The SWD engine SHALL support configurable clock frequency from 1 MHz to 4 MHz.

#### Scenario: Set 1 MHz clock
- **WHEN** probe_set_swclk_freq(1000) is called
- **THEN** PIO clock divider is configured for 1 MHz SWCLK

#### Scenario: Set 4 MHz clock
- **WHEN** probe_set_swclk_freq(4000) is called
- **THEN** PIO clock divider is configured for 4 MHz SWCLK

### Requirement: SWD Line Reset
The SWD engine SHALL perform line reset sequence (50+ high cycles).

#### Scenario: Line reset
- **WHEN** swd_line_reset() is called
- **THEN** SWDIO is driven high for at least 50 SWCLK cycles followed by 2 idle cycles
