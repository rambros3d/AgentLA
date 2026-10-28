## ADDED Requirements

### Requirement: Nordic Semi & WCH Target Detection
The system SHALL extract Nordic Semi and WCH designer IDs and part IDs during SWD target discovery.

#### Scenario: Detect nRF52840 target
- **WHEN** SWD DPIDR bits [31:12] contain Nordic Semi designer ID (0x244) or nRF52840 part ID
- **THEN** system identifies target as nRF52840 and registers nRF52840 flash algorithm

#### Scenario: Detect CH32 target
- **WHEN** SWD DPIDR or DBGMCU registers match WCH CH32F1/CH32V003 part ID
- **THEN** system identifies target as CH32 and registers CH32 flash algorithm
