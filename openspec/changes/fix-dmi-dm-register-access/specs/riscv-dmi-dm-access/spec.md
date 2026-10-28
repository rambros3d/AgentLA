## ADDED Requirements

### Requirement: DMI Register Access via CSW/TAR/DRW
The system SHALL access RISC-V DMI registers using the CSW/TAR/DRW memory-mapped pattern: write AP_CSW for 32-bit transfer size, write AP_TAR with the DMI byte address (DMI_reg * 4), then read/write AP_DRW for data.

#### Scenario: Read DMSTATUS register
- **WHEN** riscv_dmi_read(0x11, &val) is called
- **THEN** system writes AP_CSW=0x22, AP_TAR=0x44, then reads AP_DRW and returns the 32-bit DMSTATUS value

#### Scenario: Write DMCONTROL register
- **WHEN** riscv_dmi_write(0x10, 0x80000001) is called
- **THEN** system writes AP_CSW=0x22, AP_TAR=0x40, then writes AP_DRW with the value 0x80000001

#### Scenario: Read SBA registers
- **WHEN** riscv_dmi_read(0x3C, &val) is called
- **THEN** system writes AP_CSW=0x22, AP_TAR=0xF0, then reads AP_DRW and returns SBDATA0 value

### Requirement: DP Pipeline Flush After Bank Select
The system SHALL read DP_RDBUFF after writing DP_SELECT to flush the DP pipeline before the next AP access.

#### Scenario: Bank select followed by AP read
- **WHEN** swd_read_ap() is called with a register in a different bank than the previous access
- **THEN** system writes DP_SELECT, reads DP_RDBUFF (discarding the value), then performs the AP access

### Requirement: Diagnostic DMI Access Test
The system SHALL provide a diagnostic function that tests multiple DMI access methods and reports results via printf.

#### Scenario: Diagnostic reports access method results
- **WHEN** riscv_debug_init() is called
- **THEN** system tests direct AP access, CSW/TAR/DRW access, and MEM-AP access for DMSTATUS, printing ACK and value for each method

### Requirement: Halt Reliability
The system SHALL successfully halt the RISC-V core by writing DMCONTROL.haltreq and polling DMSTATUS.allhalted.

#### Scenario: Successful halt
- **WHEN** riscv_halt() is called on a connected CH32V203
- **THEN** system writes DMCONTROL with haltreq|dmactive, polls DMSTATUS until allhalted is set, and returns CH32_OK within 10ms

#### Scenario: Halt timeout
- **WHEN** riscv_halt() is called and the core does not halt within 100ms
- **THEN** system returns CH32_ERR_TIMEOUT with diagnostic output

### Requirement: Flash Operations Require Halt
The system SHALL halt the RISC-V core before performing flash erase or program operations.

#### Scenario: Flash erase with halt
- **WHEN** ch32_flash_erase() is called
- **THEN** system calls riscv_halt() first, and returns error if halt fails

#### Scenario: Flash program with halt
- **WHEN** ch32_flash_program() is called
- **THEN** system calls riscv_halt() first, and returns error if halt fails
