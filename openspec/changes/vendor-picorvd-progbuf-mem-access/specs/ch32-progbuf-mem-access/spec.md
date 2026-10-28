## ADDED Requirements

### Requirement: PROGBUF combined get/set memory access
The system SHALL provide a single combined PROGBUF memory-access program for aligned 32-bit reads and writes, ported from PicoRVD's `prog_get_set_u32`, using DATA1 bit 0 as the direction flag and clobbering only A0/A1.

#### Scenario: Aligned 32-bit read
- **WHEN** the system reads an aligned 32-bit memory location via abstract commands
- **THEN** it SHALL load the vendored get/set program into PROGBUF0-7
- **AND** write the target address into DATA1 with bit 0 clear
- **AND** execute via `POSTEXEC`
- **AND** return the value read into DATA0
- **AND** SHALL NOT modify any core GPR other than A0/A1

#### Scenario: Aligned 32-bit write
- **WHEN** the system writes an aligned 32-bit value via abstract commands
- **THEN** it SHALL load the vendored get/set program into PROGBUF0-7
- **AND** write the data into DATA0
- **AND** write the target address with bit 0 set into DATA1
- **AND** execute via `POSTEXEC`
- **AND** SHALL NOT modify any core GPR other than A0/A1

#### Scenario: Program size
- **WHEN** the get/set program is loaded
- **THEN** the program SHALL be exactly 16 halfwords (32 bytes) to fit in PROGBUF0-7

### Requirement: Per-family DTM DATA0/DATA1 magic offsets
The system SHALL use the correct WCH DTM DATA0/DATA1 magic offsets for the active target chip family, via a single lookup keyed on `g_ch32_target.chip_type` — the offsets are a DTM/chip property and SHALL NOT be keyed on the transport mode (SWIO vs RVSWD).

#### Scenario: CH32V003 offsets
- **WHEN** `g_ch32_target.chip_type` is `CH32_CHIP_V003`
- **THEN** the DTM DATA0/DATA1 magic offsets SHALL be `0xF4`/`0xF8` (PicoRVD's original CH32V003 values)

#### Scenario: CH32V20x/V30x offsets
- **WHEN** `g_ch32_target.chip_type` is `CH32_CHIP_V203` or `CH32_CHIP_V307`
- **THEN** the DTM DATA0/DATA1 magic offsets SHALL be `0x380`/`0x384` (derived from HARTINFO, matching the existing `ch32_progbuf.c` patch)

#### Scenario: Transport independence
- **WHEN** a memory access is performed via PROGBUF
- **THEN** the offset lookup SHALL be selected by chip family only, and the transport (SWIO for 1-wire, RVSWD for 2-wire) SHALL be selected by `g_ch32_target.mode`

### Requirement: ABSTRACTAUTO-pipelined block memory access
The system SHALL support block-aligned memory reads and writes ported from PicoRVD's `get_block_aligned`/`set_block_aligned`, using `ABSTRACTAUTO_AUTOEXECDATA` so each DATA0 access re-executes the program for the next word.

#### Scenario: Block-aligned read pipeline
- **WHEN** the system reads N aligned 32-bit words
- **THEN** it SHALL load the block-read program into PROGBUF
- **AND** write the start address into DATA1
- **AND** execute the first word via `POSTEXEC`
- **AND** enable `ABSTRACTAUTO_AUTOEXECDATA` so each DATA0 read produces the next word
- **AND** disable auto-execution before the final read

#### Scenario: Block-aligned write pipeline
- **WHEN** the system writes N aligned 32-bit words
- **THEN** it SHALL load the block-write program into PROGBUF
- **AND** write the start address into DATA1
- **AND** write each word into DATA0
- **AND** enable `ABSTRACTAUTO_AUTOEXECDATA` after the first word so each subsequent DATA0 write stores the next word
- **AND** disable auto-execution after the final word

### Requirement: Vended attribution
The system SHALL carry license attribution for the ported PicoRVD program tables.

#### Scenario: Attribution header
- **WHEN** the ported program tables are compiled into the firmware
- **THEN** the source file SHALL include a header crediting PicoRVD (`https://github.com/aappleby/picorvd`) under the MIT License

### Requirement: Hardware verification on real silicon
The change SHALL NOT be considered complete until hardware verification passes on real CH32 devices.

#### Scenario: CH32V003 read, flash, and debug over SWIO
- **WHEN** the firmware is flashed to an AgentLA probe connected to a CH32V003 via single-wire SWIO
- **THEN** the system SHALL detect the target, read ESIG/UID registers, program flash at `0x08000000`, and verify readback using the vendored block-aligned read
- **AND** the system SHALL halt, single-step, read/write registers, and resume the core over SWIO

#### Scenario: CH32V203/V307 readback parity over RVSWD
- **WHEN** the firmware is flashed to an AgentLA probe connected to a CH32V203/V307 via 2-wire RVSWD
- **THEN** flash readback parity SHALL match the pre-change baseline using the `0x380`/`0x384` offsets

#### Scenario: Completion gate
- **WHEN** the change is proposed for completion/archival
- **THEN** all hardware verification tasks SHALL be marked done, with the hardware and toolchain used recorded
