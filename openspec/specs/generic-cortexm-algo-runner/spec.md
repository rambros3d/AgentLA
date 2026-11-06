# generic-cortexm-algo-runner

## Purpose
Enables executing CMSIS-Pack and vendor flash algorithms directly in target RAM via SWD core debug registers (DHCSR, DCRSR, DCRDR), orchestrating Init, EraseSector, ProgramPage, Verify, and UnInit execution with explicit consent controls.

## Requirements

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

### Requirement: LittleFS algorithm blob resolution
The system SHALL resolve and load flash algorithm binaries directly from LittleFS Partition 1 (`SYSTEM_FS`) under the `/system/algo/` directory path.

#### Scenario: Load algorithm blob from storage
- **WHEN** a flash operation is requested for a target whose algorithm blob exists at `/system/algo/<name>.bin`
- **THEN** the system reads the blob from LittleFS, validates the `AlgoBlobHeader` magic (`0x4F474C41`) and version, downloads the Thumb code into target SRAM, and executes the algorithm routines

#### Scenario: Algorithm blob not found
- **WHEN** no matching algorithm blob is found in LittleFS storage
- **THEN** the system returns `ERR ALGO NOT FOUND` and performs no memory writes to target flash

### Requirement: Host algorithm packager
The system SHALL provide an offline host packager tool `tools/pack_flm.py` to extract code and entry offsets from standard Keil `.FLM` or ELF binaries into the `AlgoBlobHeader` binary format.

#### Scenario: Pack FLM to binary
- **WHEN** the packager tool is invoked with a valid `.FLM` or ELF file
- **THEN** it generates a `.bin` file prefixed with a 44-byte `AlgoBlobHeader` containing `Init`, `EraseSector`, `ProgramPage`, and `UnInit` offsets and SRAM stack layout
