## ADDED Requirements

### Requirement: RawCapture supports Sigrok export
The Python SDK SHALL allow users to export a raw capture as a Sigrok session file (`.sr` directory) that PulseView can open.

#### Scenario: Export raw capture to Sigrok format
- **WHEN** user calls `raw_capture.export_sigrok("capture.sr")`
- **THEN** SDK creates a `.sr` directory with valid `metadata` and `logic` protobuf files

#### Scenario: Sigrok export includes sample rate metadata
- **WHEN** `export_sigrok()` is called after a capture at 10 MHz
- **THEN** the session's metadata file records `10000000` Hz as the sample rate

#### Scenario: Sigrok export includes channel count
- **WHEN** `export_sigrok()` is called after capturing 8 channels
- **THEN** the session's metadata file records 8 channels with numeric names

#### Scenario: Export to existing directory
- **WHEN** `export_sigrok("capture.sr")` is called and `capture.sr/` already exists
- **THEN** SDK overwrites the existing directory contents

### Requirement: Sigrok export produces PulseView-compatible output
The exported `.sr` session SHALL be openable in PulseView without errors.

#### Scenario: Open in PulseView
- **WHEN** user opens the exported `.sr` directory in PulseView
- **THEN** PulseView displays raw pin waveforms and accepts Sigrok decoder plugins

### Requirement: SigrokDecoder class for programmatic decoding
The Python SDK SHALL provide a `SigrokDecoder` class that wraps `libsigrokdecode` for host-side protocol decoding.

#### Scenario: Decode I2C with Sigrok decoder
- **WHEN** user creates `SigrokDecoder("I2C")` and decodes raw capture data
- **THEN** SDK returns structured I2C events consistent with in-device decode output

#### Scenario: SigrokDecoder requires optional dependency
- **WHEN** user imports `SigrokDecoder` without `sigrokdecode` installed
- **THEN** SDK raises `ImportError` with clear message about installing `pip install agentla[sigrok]`

#### Scenario: Decode unsupported protocol
- **WHEN** user creates `SigrokDecoder("UNSUPPORTED_PROTOCOL")`
- **THEN** SDK raises `ValueError` listing supported protocol names from Sigrok

### Requirement: Optional `sigrok` extras dependency group
The SDK SHALL install `sigrokdecode` Python package when the `sigrok` extra is specified.

#### Scenario: Install with sigrok extras
- **WHEN** user runs `pip install agentla[sigrok]`
- **THEN** `sigrokdecode` Python package and its dependencies are installed

## MODIFIED Requirements

### Requirement: RawCapture.export_sigrok()
The Python SDK SHALL implement `export_sigrok()` as a working method instead of raising `NotImplementedError`.

#### Scenario: export_sigrok produces .sr directory
- **WHEN** user calls `raw_capture.export_sigrok("session.sr")`
- **THEN** a directory `session.sr/` is created containing `metadata` and `logic` files

#### Scenario: export_sigrok raises on invalid filename
- **WHEN** `export_sigrok("")` is called with an empty string
- **THEN** SDK raises `ValueError` with message about invalid filename