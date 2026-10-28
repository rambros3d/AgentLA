## MODIFIED Requirements

### Requirement: SWD Write Bits
The SWD engine SHALL write 1-32 bits to SWDIO with SWCLK generation using PIO seq_out sub-programs.

#### Scenario: Write 8-bit request
- **WHEN** swd_write_bits(8, request) is called
- **THEN** 8 bits are clocked out on SWDIO via PIO seq_out with correct SWCLK toggling

#### Scenario: Write with turnaround
- **WHEN** swd_write_bits is called while current_swdio_dir is FLOAT
- **THEN** PIO seq_out_turnaround sub-program SHALL be used to switch SWDIO to output before data

### Requirement: SWD Read Bits
The SWD engine SHALL read 1-32 bits from SWDIO using PIO seq_in sub-programs with autopush.

#### Scenario: Read 3-bit ACK
- **WHEN** swd_read_bits(3) is called during ACK phase
- **THEN** PIO seq_in sub-program reads 3 bits and push instruction outputs ISR to RX FIFO

#### Scenario: Read 32-bit data
- **WHEN** swd_read_bits(32) is called during data phase
- **THEN** 32 bits are read and autopush fires at threshold to push ISR to RX FIFO

### Requirement: SWD Transfer Buffer Structure
The swd_transfer() function SHALL build PIO buffers matching the MioLink reference pattern from swdtap_adiv5_prepare_pio_seq().

#### Scenario: Write transfer combines data+parity
- **WHEN** swd_transfer() builds a write PIO buffer
- **THEN** data (32 bits) + parity (1 bit) + idle cycles SHALL be packed into a single seq_out operation with bit count = 32 + 1 + IDLE - 1

#### Scenario: Write transfer has no SWD_ORIGIN_START
- **WHEN** swd_transfer() builds a write PIO buffer
- **THEN** the buffer SHALL NOT contain SWD_ORIGIN_START at the end

#### Scenario: Read transfer has no SWD_ORIGIN_START
- **WHEN** swd_transfer() builds a read PIO buffer
- **THEN** the buffer SHALL end with SWD_ORIGIN_TURNAROUND_TO_DRIVE followed by SWD_ORIGIN_START only for idle cycles

#### Scenario: Idle cycles included in writes
- **WHEN** swd_transfer() builds a write PIO buffer
- **THEN** TARGET_SWD_IDLE_CYCLES (default 4) SHALL be included in the combined data+parity bit count
