## MODIFIED Requirements

### Requirement: RISC-V Debug Module Interface (DMI) Access
The system SHALL read and write DMI registers over 2-wire RVSWD and 1-wire SWIO transports, selected via `g_ch32_target.mode` (`CH32_MODE_RVSWD` / `CH32_MODE_SWIO`); the transport SHALL be selected independently of the chip-family-keyed DTM offset lookup.

#### Scenario: DMCONTROL register access
- **WHEN** core halt or resume is requested
- **THEN** system writes `DMCONTROL` (`0x10`) register to control CPU execution state

#### Scenario: Abstract command execution
- **WHEN** CPU register or memory read is issued
- **THEN** system executes RISC-V abstract commands via `COMMAND` (`0x17`) and `DATA0` (`0x04`)
- **AND** aligned memory accesses SHALL use the vendored combined get/set PROGBUF program (clobbering only A0/A1, direction via DATA1 bit 0) instead of separate per-direction programs
- **AND** block accesses SHALL use `ABSTRACTAUTO_AUTOEXECDATA` pipelining
