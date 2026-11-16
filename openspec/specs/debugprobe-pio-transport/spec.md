# debugprobe-pio-transport Specification

## Purpose
TBD - created by archiving change import-debugprobe-pio-swd. Update Purpose after archive.
## Requirements
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

### Requirement: Ported SWD_Transfer transaction
The system SHALL provide a PIO SWD transaction engine ported from debugprobe's `SWD_Transfer` that performs: request phase (8 bits out), ACK read as `turnaround + 3` bits with the first `turnaround` bits discarded, data phase on ACK OK (read 32+1 with parity verification, or hi-Z turnaround then write 32+1), WAIT/FAULT data-phase back-off (dummy read of 33 bits or dummy write of 32+1 bits), protocol-error re-sync (read `turnaround + 32 + 1` bits), and idle cycles after the transfer.

#### Scenario: Read transaction succeeds
- **WHEN** a read request yields ACK OK
- **THEN** 32 data bits and 1 parity bit are read, the parity is verified against the data, and the data word is returned with ACK OK

#### Scenario: Write transaction succeeds
- **WHEN** a write request yields ACK OK
- **THEN** after a hi-Z turnaround, 32 data bits and 1 parity bit are written, followed by idle cycles, and ACK OK is returned

#### Scenario: WAIT or FAULT data-phase back-off
- **WHEN** a transaction yields ACK WAIT or FAULT
- **THEN** a dummy data phase (read 33 bits or write 32+1 bits) is consumed before returning, leaving the line in a defined state for the next transaction

#### Scenario: Protocol error re-sync
- **WHEN** the ACK is neither OK, WAIT, nor FAULT
- **THEN** the engine reads `turnaround + 32 + 1` bits to back off the data phase and returns the error

### Requirement: SWD clock frequency control
The system SHALL set the SWD clock frequency via the debugprobe transport's clock divider (`probe_set_swclk_freq`), mapping the requested kHz to an integer SM clock divider derived from `CPU_CLOCK`, with clamping to the `SWD_FREQ_MIN`/`SWD_FREQ_MAX` limits.

#### Scenario: Set default 100 kHz
- **WHEN** the PIO transport is initialized
- **THEN** the SM clock divider SHALL be set for 100 kHz SWCLK

#### Scenario: Clamp out-of-range frequencies
- **WHEN** a requested frequency is below `SWD_FREQ_MIN` or above `SWD_FREQ_MAX`
- **THEN** the frequency SHALL be clamped to the limit

### Requirement: DP power-up request value
The system SHALL request debug and system power-up by writing `0x30000000` (CDBGPWRUPREQ | CSYSPWRUPREQ, bits 28 and 29) to DP CTRL/STAT, and SHALL use this value at all power-up call sites in both the PIO and bitbang branches.

#### Scenario: Power-up after connect
- **WHEN** the DP is connected and the first DPIDR read has succeeded
- **THEN** the ABORT register is written with `0x1E` and CTRL/STAT with `0x30000000`

### Requirement: Vendored attribution
The vendored debugprobe transport files SHALL retain their upstream license headers (MIT for `probe.pio`/`probe.c` derivatives, Apache-2.0 for `SWD_Transfer`), and SHALL note the pinned upstream commit and any local adaptations.

#### Scenario: License headers preserved
- **WHEN** the vendored transport files are checked in
- **THEN** each file SHALL contain its upstream license/copyright header and an annotation of the source commit and local modifications

