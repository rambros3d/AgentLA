## Context

AgentLA’s firmware command surface is already large, but only a limited set is visible to network clients. The WebUI currently creates local windows and generic command text without a consistent operation contract. This design defines a browser-usable API on top of existing command dispatch rather than duplicating hardware logic.

## Goals / Non-Goals

**Goals:**
- Provide stable command discovery and execution via HTTP/RNDIS.
- Allow WebUI controls to pass entered values.
- Support long operations through handles and progress events.
- Preserve command ownership and safety boundaries.
- Reconcile task overreporting in the archived WebGUI change.
- Avoid claiming Core 1 behavior that does not yet exist.

**Non-Goals:**
- Full NetAPI/OpenAPI schema generation.
- Hardware-level virtualization.
- Unbounded data transfer performance.
- Full security/auth hardening beyond USB-local trust.

## Decisions

### 1. Command String Bridge
Reuse existing text commands. API methods translate into canonical command strings and dispatcher calls.

### 2. Safety Classes
Commands use four visible classes: `read`, `setup`, `adapter`, `destructive`. Diagnostics remain explicitly marked.

### 3. Operation Handles
Long operations return `handle`, followed by SSE progress keyed by that handle.

### 4. UI Form Contract
Each app window collects its fields and maps them to one canonical command rather than inventing new endpoints.

### 5. Core Policy
WebUI and command/status workflows run on Core 0. Capture execution only remains on Core 0 until Core 1 is actually launched.

### 6. Status Reconciliation
The next change implementation does not inherit the “complete” flags from the archived WebGUI change.

## Risks / Trade-offs
- [Risk] Browser fetch and lwIP tiny buffers can truncate payloads → enforce chunking and explicit max.
- [Risk] Synchronous command dispatch can block WebUI → use operation handles for long tasks.
- [Risk] UI forms may pass invalid state → rely on command parser and structured responses.

## Migration Plan
1. Stabilize bridge and discovery.
2. Add operation handle lifecycle.
3. Wire Flasher first.
4. Expand UI controls and diagnostics after a verified loop.
5. Run high-level smoke tests with build artifacts.