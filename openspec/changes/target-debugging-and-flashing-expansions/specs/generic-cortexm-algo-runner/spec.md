## ADDED Requirements

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
