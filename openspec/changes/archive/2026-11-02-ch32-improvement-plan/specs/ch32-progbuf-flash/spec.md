## ADDED Requirements

### Requirement: PROGBUF-based flash write for CH32V003
The system SHALL implement PROGBUF injection for CH32V003 flash programming, streaming data through the debug module without consuming target RAM.

#### Scenario: Flash write via PROGBUF
- **WHEN** the user issues `CH32 FLASH 0x08000000 <hex_data>` for a CH32V003 target
- **THEN** the system SHALL load a RISC-V flash write routine into PROGBUF0-7
- **AND** configure GPR registers (a0-a5) via abstract commands
- **AND** enable AUTOCMD (abstractauto=1) for auto-execution
- **AND** stream 32-bit words through DATA0 at full debug interface speed
- **AND** the PROGBUF program SHALL handle BUFLOAD, page boundary detection, and page program trigger

#### Scenario: PROGBUF flash program content
- **WHEN** loading the flash write routine into PROGBUF
- **THEN** the program SHALL be exactly 16 halfwords (32 bytes) to fit in PROGBUF0-7
- **AND** the program SHALL: load word from DATA0 address, store to flash buffer, trigger BUFLOAD, check BSY flag, detect page boundary, trigger page program, reset buffer

#### Scenario: PROGBUF flash without target RAM
- **WHEN** PROGBUF injection is active
- **THEN** the system SHALL NOT write any data to target RAM
- **AND** data SHALL flow directly from host through DATA0 to flash page buffer

### Requirement: PROGBUF flash fallback
The system SHALL fall back to simple word-at-a-time writes if PROGBUF injection fails.

#### Scenario: PROGBUF initialization failure
- **WHEN** PROGBUF write verification fails (readback mismatch)
- **THEN** the system SHALL fall back to the existing `riscv_write_mem()` approach
- **AND** log a warning about fallback mode

### Requirement: Flash verify after write
The system SHALL verify flash contents after programming.

#### Scenario: Flash verification
- **WHEN** flash write completes successfully
- **THEN** the system SHALL read back the written data via System Bus Access (SBA)
- **AND** compare against the original data
- **AND** report success or mismatch with address offset
