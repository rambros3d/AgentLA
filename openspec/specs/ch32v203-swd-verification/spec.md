## ADDED Requirements

### Requirement: CH32V203 RV-SWD Hardware Link Verification
The test suite SHALL verify 2-wire RV-SWD physical layer communication and chip electronic signature (ESIG) detection on CH32V203.

#### Scenario: CH32V203 Target Probe
- **WHEN** target detection command `CH32 DETECT mode=2wire` is issued to AgentLA
- **THEN** system initializes SWD, reads ESIG at `0x1FFFF704`, and reports target status `OK CH32 DETECT`

### Requirement: CH32V203 Flash Erase and Programming Verification
The test suite SHALL verify fast page unlock, sector erase, and payload page programming on CH32V203 targets.

#### Scenario: Page Erase and Fast Write Verification
- **WHEN** binary payload is flashed to address `0x08000000` via `CH32 FLASH` and read back via `FLASH READ`
- **THEN** written memory contents match the target payload byte-for-byte with zero mismatches
