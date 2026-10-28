## ADDED Requirements

### Requirement: SigrokDecoder class provides programmatic protocol decoding
The Python SDK SHALL provide a `SigrokDecoder` class that wraps `libsigrokdecode` for host-side protocol decoding of raw capture data.

#### Scenario: Decode I2C with Sigrok decoder
- **WHEN** user creates `SigrokDecoder("I2C")` and calls `decode(raw_data, sample_rate)`
- **THEN** SDK returns a list of `SigrokEvent` objects with `type`, `timestamp`, and `data` fields

#### Scenario: SigrokDecoder lazy-loads libsigrokdecode
- **WHEN** `SigrokDecoder` is instantiated
- **THEN** SDK imports `sigrokdecode` only at instantiation time, not at module load

#### Scenario: SigrokDecoder raises on missing dependency
- **WHEN** `SigrokDecoder` is instantiated without `sigrokdecode` package installed
- **THEN** SDK raises `ImportError("sigrokdecode package required: pip install agentla[sigrok]")`

#### Scenario: Decode SPI with Sigrok decoder
- **WHEN** user creates `SigrokDecoder("SPI")` and decodes raw capture
- **THEN** SDK returns structured SPI events matching Sigrok's decoder output format

#### Scenario: Decode unsupported protocol
- **WHEN** user creates `SigrokDecoder("UNSUPPORTED_PROTOCOL")`
- **THEN** SDK raises `ValueError` with message indicating the protocol is not available in Sigrok

### Requirement: SigrokDecoder output is compatible with AgentLA event types
The `SigrokDecoder.decode()` output SHALL be structured to be compatible with AgentLA's existing `I2CEvent`, `SPIEvent`, and `UARTEvent` dataclasses.

#### Scenario: SigrokDecoder I2C output maps to I2CEvent
- **WHEN** SigrokDecoder decodes I2C raw data
- **THEN** output events have compatible fields: `timestamp`, `type`, `value`, `rw`

#### Scenario: SigrokDecoder output includes protocol name
- **WHEN** decoder produces events
- **THEN** each event includes `protocol` field identifying the decoder used