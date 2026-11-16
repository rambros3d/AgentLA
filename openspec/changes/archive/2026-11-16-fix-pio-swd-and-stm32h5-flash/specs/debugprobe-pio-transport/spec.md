## MODIFIED Requirements

### Requirement: PIO transport primitives
The system SHALL provide PIO-based SWD bit primitives vendored from Raspberry Pi debugprobe (`probe_write_bits`, `probe_read_bits`, `probe_hiz_clocks`) that drive SWDIO on GPIO5 and generate SWCLK on GPIO4 via PIO0 SM0 with `sideset_count` strictly configured to 1 (`SWCLK` only), with the state machine using explicit `pull` for output data and explicit `push` for captured data, autopull and autopush disabled, direction encoded in-band per command word without sideset interference on SWDIO, and unconditionally generating at least 2 trailing idle clock cycles (`write_bits(0, 2)`) following transaction completion.

#### Scenario: Write bits with continuous clock
- **WHEN** `probe_write_bits(n, data)` is called with n ≤ 32
- **THEN** the SM outputs n bits LSB-first on SWDIO while SWCLK runs continuously without sideset driving SWDIO, and the function returns without waiting for completion

#### Scenario: Read bits with continuous clock
- **WHEN** `probe_read_bits(n)` is called
- **THEN** the SM clocks n bits, captures SWDIO without sideset driving SWDIO, and the function blocks until the captured word is returned via explicit push

#### Scenario: Hi-Z clock cycles
- **WHEN** `probe_hiz_clocks(n)` is called
- **THEN** the SM clocks n cycles with SWDIO released (input mode) for turnaround

#### Scenario: Error-free DP write transfer
- **WHEN** a DP write transfer (`DP_SELECT`, `DP_CTRLSTAT`) is executed over SWD
- **THEN** the target acknowledges with `SWD_ACK_OK` (`ack=1`), and `DP_CTRLSTAT` bit 7 (`WDATAERR`) remains zero

#### Scenario: Trailing idle clock cycles
- **WHEN** an SWD transfer completes (read or write)
- **THEN** the system clocks out at least 2 idle zero bits to flush the target CoreSight DP pipeline before releasing or starting the next transaction
