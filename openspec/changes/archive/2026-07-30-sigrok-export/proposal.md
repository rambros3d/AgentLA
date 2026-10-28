## Why

AgentLA's Python SDK has a stub `export_sigrok()` method on `RawCapture` that raises `NotImplementedError`. Users capturing raw data have no way to export session files for PulseView/Sigrok analysis. This blocks the human-in-the-loop workflow: agents can capture and decode, but they can't produce artifacts that human engineers can inspect in PulseView.

## What Changes

- Implement `RawCapture.export_sigrok(filename)` to write valid Sigrok session files (`.sr` directory format) using protobuf serialization — zero new Python dependencies
- Add optional `SigrokDecoder` class in a new `sigrok` integration module, gated behind a `python-sdk[sigrok]` extra dependency — lets agents programmatically run Sigrok's 200+ decoders on captured raw data
- Update `pyproject.toml` to include `[sigrok]` optional dependency group
- Add Python SDK tests for Sigrok export and decoder functionality

## Capabilities

### New Capabilities
- `sigrok-export`: SDK capability to export raw capture data as Sigrok session files (`.sr`) for PulseView visualization
- `sigrok-decoder`: SDK capability to run Sigrok protocol decoders via `libsigrokdecode` on captured raw data, gated as optional dependency

### Modified Capabilities
- `python-sdk`: Extends `RawCapture` class with `export_sigrok()` method; adds `SigrokDecoder` class and optional `sigrok` dependency group

## Impact

- **Host Software**: Python SDK gains Sigrok export and integrated decoding capabilities
- **Dependencies**: Core SDK keeps zero new deps (protobuf already available); `sigrok` extra adds `sigrokdecode` pip package and requires `libsigrokdecode` C library on host
- **Files Modified**: `sdk/agentla/capture.py`, `sdk/agentla/sigrok.py` (new), `sdk/pyproject.toml`, `sdk/README.md`
- **Tests**: New test file `test/sigrok_export_test.py`