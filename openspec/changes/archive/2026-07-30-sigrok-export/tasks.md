## 1. Sigrok Export Implementation

- [x] 1.1 Implement `RawCapture.export_sigrok(filename)` method using protobuf serialization
- [x] 1.2 Write Sigrok session directory format (metadata + logic protobuf files)
- [x] 1.3 Add validation for filenames and directory creation
- [x] 1.4 Write unit tests for export output format and file structure

## 2. SigrokDecoder Optional Integration

- [x] 2.1 Create `sdk/agentla/sigrok.py` with `SigrokDecoder` class
- [x] 2.2 Implement lazy loading of `sigrokdecode` package with clear ImportError
- [x] 2.3 Map Sigrok decoder output to AgentLA event dataclasses (I2CEvent, SPIEvent, UARTEvent)
- [x] 2.4 Add `[sigrok]` optional dependency group to `pyproject.toml`

## 3. Testing and Validation

- [x] 3.1 Write integration test: capture_raw → export_sigrok → verify .sr directory is valid
- [x] 3.2 Write test: SigrokDecoder I2C decode matches agent-decoded I2C events
- [x] 3.3 Add README section for Sigrok integration usage
- [x] 3.4 Test error handling: missing sigrokdecode, invalid filenames, corrupt data

## 4. Documentation

- [x] 4.1 Update SDK README with Sigrok export and decoder usage examples
- [x] 4.2 Document `[sigrok]` extra installation and system dependencies
- [x] 4.3 Add usage examples for both export and decoder paths