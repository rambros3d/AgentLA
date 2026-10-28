## ADDED Requirements

### Requirement: RawCapture.export_sigrok()
The SDK SHALL implement `RawCapture.export_sigrok(filename)` to write a valid Sigrok `.sr` session directory from raw capture data.

#### Scenario: Export produces .sr directory
- **WHEN** user calls `raw_capture.export_sigrok("capture.sr")`
- **THEN** SDK creates `capture.sr/` directory with `metadata` and `logic` protobuf files

#### Scenario: Export includes sample rate
- **WHEN** `export_sigrok()` is called after capture at 10 MHz
- **THEN** the metadata file records `sample_rate = 10000000`

#### Scenario: Export includes channel information
- **WHEN** `export_sigrok()` is called after 8-channel capture
- **THEN** metadata includes channel count and numeric channel names

#### Scenario: Export to existing directory overwrites
- **WHEN** `export_sigrok("capture.sr")` called on existing directory
- **THEN** SDK overwrites `metadata` and `logic` files

#### Scenario: Export raises on invalid filename
- **WHEN** `export_sigrok("")` called with empty string
- **THEN** SDK raises `ValueError("Invalid Sigrok session filename")`

### Requirement: Optional sigrok extras dependency group
The SDK SHALL define a `[sigrok]` optional dependency group in `pyproject.toml` that includes `sigrokdecode`.

#### Scenario: Install with sigrok extras
- **WHEN** user runs `pip install agentla[sigrok]`
- **THEN** `sigrokdecode` package and its transitive dependencies are installed