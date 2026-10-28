## Context

AgentLA is an RP2350-based logic analyzer with dual-mode operation: on-device decoded mode for LLM agents and raw capture mode for Sigrok/PulseView compatibility. The Python SDK (`agentla`) currently has a stub `export_sigrok()` method on `RawCapture` that raises `NotImplementedError`, blocking human users from generating PulseView session files from AgentLA captures.

The `.sr` (Sigrok session) format uses protobuf serialization for metadata and raw logic data. It is a directory whose contents can be written using the `protobuf` Python package (already a transitive dependency of AgentLA's SDK environment). No C library dependencies are needed for writing `.sr` files — only `libsigrokdecode` C libraries are required for runtime protocol decoding via `SigrokDecoder`.

This change implements Path A from the Sigrok integration exploration: Phase 1 (host-side export) as a zero-dependency core feature, with Phase 2 (programmatic Sigrok decoding) as an optional extras dependency.

## Goals / Non-Goals

**Goals:**
- `RawCapture.export_sigrok(filename)` writes a valid PulseView-compatible `.sr` session directory
- Zero new Python package dependencies for Phase 1 (uses protobuf and struct from stdlib + existing deps)
- Phase 2 `SigrokDecoder` class gated as `python-sdk[sigrok]` optional extra
- Agent can bundle a `.sr` file alongside its reasoning output for human review

**Non-Goals:**
- On-device Sigrok decoding (firmware doesn't run libsigrokdecode)
- Sigrok decoder integration in the firmware agent protocol
- Real-time streaming to Sigrok on the host (raw data is already serial-streamable)
- GUI integration beyond `.sr` file generation

## Decisions

### D1: `.sr` format via protobuf (not tar or custom format)

**Decision:** Write `.sr` as a directory using protobuf serialization for `metadata` and `logic` files, matching libsigrok's modern session format.

**Rationale:** PulseView reads `.sr` directories natively. Protobuf is the documented format. Using `protobuf` Python package avoids implementing a binary format from scratch. No C library needed for writing.

**Alternatives considered:**
- Tar archive `.sr` file: PulseView expects directory or specific tar layout, more complex to produce correctly
- Custom text format: Not compatible with PulseView, defeats the purpose
- Defer to `libsigrok` Python bindings: Would require C library on host, adds installation friction

### D2: `SigrokDecoder` as optional extras dependency

**Decision:** `SigrokDecoder` is in a `sigrok` optional dependency group, not required for core SDK functionality.

**Rationale:** `sigrokdecode` Python package requires `libsigrokdecode` and `libsigrok` C libraries compiled on the system. Installing these is non-trivial across platforms. Keeping it optional keeps the core SDK lightweight (currently only `pyserial`).

**Alternatives considered:**
- Hard dependency on `sigrokdecode`: Would break installation on many systems where C libraries aren't available
- Bundling decoders in AgentLA firmware: Duplicates Sigrok's maintenance burden; goes against the on-device custom-decoder approach

### D3: Export happens on `RawCapture`, not `Device`

**Decision:** `export_sigrok()` is a method on `RawCapture` (the data object), not on `Device` (the connection).

**Rationale:** `RawCapture` is a data container with sample bytes, sample rate, and channel count. The export logic is purely data transformation. Keeping it on the capture object follows the Single Responsibility Principle and makes testing easier.

**Alternatives considered:**
- `Device.export_sigrok(capture)` — adds coupling between Device and export logic
- Standalone `sigrok_export(capture, filename)` function — less discoverable in the SDK API

### D4: Minimal metadata for `.sr` compatibility

**Decision:** Write only essential metadata fields (sample rate, channel count, channel number, signal name). Skip optional fields like device model and capture settings.

**Rationale:** PulseView can open `.sr` sessions with minimal metadata. The key requirement is valid logic data that can be decoded. Extra metadata is nice-to-have but not required for basic compatibility.

**Alternatives considered:**
- Full metadata including all capture settings: More compatible with advanced PulseView features but fragile if schema changes across libsigrok versions
- No metadata at all: PulseView would reject the session as invalid

## Risks / Trade-offs

- **[Risk] Protobuf format changes across libsigrok versions** → Mitigation: Target the most common libsigrok version range (0.7.x) used in current distributions; test with PulseView
- **[Risk] `sigrokdecode` Python package is fragile across Linux distros** → Mitigation: Keep as optional extra; document the system dependency (`apt install libsigrokdecode-dev`); provide clear error messages if import fails
- **[Risk] RAW capture produces large binary files in `.sr` sessions** → Mitigation: AgentLA captures are typically small (4096 samples = a few KB); larger captures should warn users about `.sr` file size
- **[Trade-off] `SigrokDecoder` duplicates AgentLA's built-in decoders** → Acceptable: Sigrok has 200+ decoders; AgentLA has 4. For protocols AgentLA already decodes, the on-device ARM path is faster and agent-friendly. `SigrokDecoder` is for protocols AgentLA doesn't cover.

## Migration Plan

1. Existing SDK users: `RawCapture.export_sigrok()` will change from raising `NotImplementedError` to producing valid `.sr` files — no breaking change in public API, behavior change from error to success
2. Users who need `SigrokDecoder`: install with `pip install agentla[sigrok]` after ensuring `libsigrokdecode` C library is available on host
3. AgentLA v1.0 firmware already produces compatible raw capture data; no firmware changes needed for export

## Open Questions

1. **Should `export_sigrok()` compress the `.sr` directory into a tar.gz?** PulseView reads directories and tar archives; directory is simpler to implement and inspect, but tar is more portable as a single file
2. **What protobuf version do we target?** Sigrok uses protobuf 3.x; Python's `protobuf` package v6+ should be compatible for writing but may have wire-format differences for reading
3. **Should we include sample data as bit-packed or byte-packed in the `logic` file?** Sigrok uses LSB-first bit packing (1 bit per sample); byte-packing would be simpler but incompatible with PulseView's expected format