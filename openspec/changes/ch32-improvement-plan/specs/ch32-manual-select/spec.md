## MODIFIED Requirements

### Requirement: Manual MCU model selection
The system SHALL require the user to explicitly specify the CH32 MCU model (or debug interface mode) when probing a target. The MCU model determines which debug transport and flash parameters are used. Automatic detection of the debug interface is intentionally NOT performed: it produced false positives on real hardware (ESIG reads returning 0x00000000), and the user always knows which MCU they are programming.

#### Scenario: CH32V203 via `mcu=`
- **WHEN** the user issues `CH32 DETECT mcu=ch32v203`
- **THEN** the system SHALL use the RVSWD two-wire (SWD) transport to probe the target
- **AND** the system SHALL configure flash parameters for CH32V20x (64 KiB flash, 256-byte pages)
- **AND** the system SHALL report the canonical chip_id from the MCU model table

#### Scenario: CH32V003 via `mcu=`
- **WHEN** the user issues `CH32 DETECT mcu=ch32v003`
- **THEN** the system SHALL use the SWIO single-wire transport to probe the target
- **AND** the system SHALL configure flash parameters for CH32V003 (16 KiB flash, 64-byte pages)
- **AND** the system SHALL report the canonical chip_id from the MCU model table

#### Scenario: Unknown MCU model
- **WHEN** the user issues `CH32 DETECT mcu=<unknown>`
- **THEN** the system SHALL report an error listing the supported MCU models
- **AND** SHALL NOT attempt any probe

#### Scenario: Legacy mode override preserved
The user SHALL be able to explicitly specify the debug interface mode as a fallback for backward compatibility with existing test scripts.

- **WHEN** the user issues `CH32 DETECT mode=1wire`
- **THEN** the system SHALL use SWIO mode with CH32V003 default flash parameters (16 KiB, 64-byte pages)
- **WHEN** the user issues `CH32 DETECT mode=2wire`
- **THEN** the system SHALL use RVSWD two-wire mode with CH32V20x default flash parameters (64 KiB, 256-byte pages)

#### Scenario: Verified probe result
- **WHEN** a probe is attempted for any model or mode
- **THEN** the system SHALL only report success if the debug link is verified (valid DMSTATUS via the selected transport)
- **AND** the system SHALL NOT fabricate a chip_id from an unverified ESIG read (ESIG reading 0x00000000 SHALL NOT be treated as a valid chip identification)
