## Context

AgentLA currently exposes its real capabilities through a CDC text command loop in `main.c`, while the WebUI and `/mcp` layer only return hardcoded responses. The archived `webgui-mcp-agent` task plan marked extensive API and UI milestones complete, but the implementation remains a scaffold. Future work should let browsers drive the same firmware operations without duplicating command logic or breaking tests. The WebUI must also respect the safe-command surface and newer pin-ownership rules.

## Goals / Non-Goals

**Goals:**
- Make `/mcp` a real request/response bridge to the existing firmware command surface.
- Preserve command semantics and structured errors while improving them incrementally for browser use.
- Support all safe capability classes, not just five hard-coded methods.
- Support parameterized requests, job IDs, structured results, and progress streams.
- Keep execute-blocking operations from starving the network task.
- Make WebUI controls actually invoke configured commands with their entered values.
- Reconcile task status with implementation reality.

**Non-Goals:**
- Full REST/OpenAPI or JSON schema generation.
- Full security/auth model beyond USB-local RNDIS trust.
- Tight real-time latency guarantees on execution compared to CDC.
- Exhaustive WinBox animation or styling work.
- 100% command parity if it would expose unsafe developer modes.
- Deep browser-side state management beyond basic event handling.

## Decisions

### 1. Reuse `process_command()` behind a single dispatch gate

**Decision:** Define `webui_t result = webui_exec("SWD IDCODE", response_buf, buf_size)` or JSON-RPC `cmd` mapping and route it through the same central command dispatcher after parameter assembly.

**Rationale:** The command surface is already large. Duplicating logic in a network-side interpreter would immediately drift and weaken tests.

**Alternatives considered:** Implement a separate JSON command interpreter. Rejected because behavior would diverge from the USB CLI.

### 2. Bridge first, typed wrappers second

**Decision:** Begin by supporting generic `cmd` calls with method and tokenized args. Add targeted typed wrappers only for high-value operations.

**Rationale:** This delivers maximum command coverage quickly, then improves ergonomics.

### 3. Classify command and method access

**Decision:** Maintain safety classes:
- `ReadOnly`
- `SetupOrDebug`
- `AdapterOrCapture`
- `Destructive`
- `DiagnosticsOrSelfTest`

Read methods do not need guard; destructive methods require exact arguments and confirmation.

**Rationale:** The firmware and target-mcu operations vary in risk and should not all be offered as one-click buttons.

### 4. Use operation handles for long jobs

**Decision:** Commands may return `job_id`. The system runs one active job at a time and emits SSE `data: {json}` events containing its ID, state, progress, result, and run logs.

**Rationale:** Long `FLASH`, `CAPTURE`, `MASSERASE`, and recovery operations should not hold HTTP sockets open raw and should be monitorable separately.

**Alternatives considered:** Blocking synchronous HTTP responses only. Rejected because large targets, long captures, and browser reliability make this fragile.

### 5. Store parameters in normalized request packets

**Decision:** The CGI parser converts JSON-RPC `params` and query/form fields into a normalized struct that populates a bounded command string for `process_command()`.

**Rationale:** This keeps the path compact and leverages current validation.

### 6. Use POST for structured JSON / data ingestion

**Decision:** Enable `LWIP_HTTPD_SUPPORT_POST` and provide a custom handler for `POST /mcp` (and perhaps `/upload`), while retaining query GET access for browser/debug convenience.

**Rationale:** Base64 hex payloads in query parameters are fragile and hit URL limits.

### 7. SSE is a named event channel

**Decision:** Extend SSE with per-event framing and operation ID. Add subscription identity and bounded buffering. Push only generated progress events after operations acknowledge each stage.

**Rationale:** Browser network stacks and RNDIS latency require explicit streams.

### 8. WebUI controls are wired but assume ownership handles

**Decision:** Browser operations use the underlying firmware ownership model:
- Core 1 is not required for the WebAPI itself; net tasks remain on core 0;
- adapters capture handles from claims;
- UI panels map controls to method and handle parameters.

**Rationale:** This matches recent pin-ownership and adapter behavior changes, though current firmware already performs the command work in the same controller context.

### 9. Reconcile archived task statuses

**Decision:** Mark the prior `webgui-mcp-agent` milestones incomplete where implementation status is only scaffolding, then use this new change to implement and verify real behavior.

**Rationale:** Task boards should not accidentally report complete capabilities.

## Risks / Trade-offs

- [Execution in HTTP context] Synchronous handlers can affect USB and capture timing. → Keep HTTP methods short and queue proposals for long races.
- [No persisted state] Ownership state exists only for per-handle session duration. → Browser workflows need explicit claim and release.
- [Resource limit] lwIP and pico_httpd buffers are finite. → JSON payload limits, base64 chunking, and SSE buffering.
- [Auth model] USB-local trust means accidental local browser access could execute firmware commands. → CLI warning and methods classification; basic same-origin/origin checks.
- [Task mismatch] Archived task status conflicts with the code tree. → Reconcile before future graders rely on it.
- [UI complexity] Existing UI is generic. → Target a smaller first-wave tool set instead of all windows at once.

## Migration Plan

1. Add typed JSON parameter parsing and `process_command()` adapter.
2. Expose safe read commands first behind `/mcp`.
3. Add Adapter, GPIO, debugger, capture, and flash method routing.
4. Add operation queue, SSE event framing, and job status API.
5. Wire WebUI controls and transfer dialogs.
6. Add verification for JSON bridge, controls, safety classifications, and task status reconciliation.

## Open Questions

- Should `/mcp` support arbitrary command names by default, or should it expose a curated tool registry?
- Should operation handles be global per network client or probe-local?
- Should flash upload be direct binary or base64 to simplify memory handling?
- For multi-tab UI, should commands be exclusive to one owning client or force a transfer mode?