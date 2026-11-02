## ADDED Requirements

### Requirement: Generic algorithm execution
The system SHALL flash an MCU by downloading an official flash algorithm blob into target SRAM and invoking its entry points via the core debug interface.

#### Scenario: Successful generic program
- **WHEN** a valid algorithm blob and image are supplied for an unlocked target
- **THEN** the system executes Init, erase, program, verify, and UnInit, returning `OK FLASH DONE`

#### Scenario: Missing or corrupt blob
- **WHEN** the algorithm blob fails integrity checks
- **THEN** the system returns `ERR ALGO INVALID` before any target memory is written

### Requirement: Algorithm ABI conformance
The system SHALL require blob metadata describing Init/EraseSector/ProgramPage/Verify/UnInit entry offsets, SRAM window, and stack location.

#### Scenario: Incomplete metadata
- **WHEN** required metadata fields are missing
- **THEN** the system rejects the operation with `ERR ALGO METADATA`

### Requirement: Consent for target-side code
The system SHALL require explicit user consent before downloading and executing a target-side algorithm.

#### Scenario: Consent not given
- **WHEN** an algorithm operation is requested without consent
- **THEN** the system returns `ERR CONSENT REQUIRED` and performs no target action

### Requirement: Algorithms from official packs only
The system SHALL document and require that only official vendor/CMSIS-pack algorithms be used for target-side execution.

#### Scenario: Documentation present
- **WHEN** a user consults the programming documentation
- **THEN** official-pack provenance is specified as a prerequisite for algorithm execution