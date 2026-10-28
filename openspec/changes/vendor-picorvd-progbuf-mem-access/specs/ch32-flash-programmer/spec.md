## MODIFIED Requirements

### Requirement: WCH Target Electronic Signature Detection
The system SHALL identify connected CH32 microcontrollers using ESIG and IDCODE registers.

#### Scenario: CH32V003 probe
- **WHEN** probe target is called on CH32V003
- **THEN** IDCODE at `0x1FFFF7C4` is read and matched against known device IDs

#### Scenario: Fast Page Flash Programming
- **WHEN** binary data is flashed to CH32 target
- **THEN** fast mode keys unlock `FLASH_MODEKEYR` and page buffers are loaded before committing write

#### Scenario: Flash readback verification
- **WHEN** flash contents are verified after programming
- **THEN** the system SHALL read back the written data using the vendored `get_block_aligned` fast-read path (via `ABSTRACTAUTO_AUTOEXECDATA`) rather than word-at-a-time reads
