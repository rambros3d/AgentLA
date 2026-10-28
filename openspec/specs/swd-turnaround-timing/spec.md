# swd-turnaround-timing Specification

## Purpose
TBD - created by archiving change rewrite-swd-pio-engine. Update Purpose after archive.
## Requirements
### Requirement: SWD turnaround timing matches PIO hardware

The SWD turnaround period shall be handled by dedicated PIO sub-programs, not by C code assumptions about cycle counts.

#### Scenario: Write-to-read turnaround
- **WHEN** a read request follows a write operation
- **AND** the SWDIO direction must switch from output to input
- **THEN** the PIO shall execute the `seq_in_turnaround` sub-program
- **AND** SWDIO shall be switched to input mode before the first read bit

#### Scenario: Read-to-write turnaround
- **WHEN** a write request follows a read operation
- **AND** the SWDIO direction must switch from input to output
- **THEN** the PIO shall execute the `seq_out_turnaround` sub-program
- **AND** SWDIO shall be switched to output mode before the first write bit

#### Scenario: Turnaround without data transfer
- **WHEN** a turnaround is needed without a data sequence
- **THEN** the PIO shall execute the `turnaround_float_to_drive` sub-program
- **AND** the direction shall be switched without clocking data bits

