## Why

AgentLA’s network and WebUI foundation can connect, serve static pages, and negotiate an RNDIS link, but it does not yet execute firmware commands. Its `/mcp` endpoint returns canned responses, and the WinBox buttons call coarse methods that are not wired to the probe’s command surface. As a result, the network UI cannot actually flash, debug, configure capture, or operate adapter and debugger functions.

## What Changes

- Add a JSON-RPC command bridge that exposes firmware command families through the HTTP/RNDIS interface as typed requests.
- Route `/mcp` requests to internal command execution on the controlling core, preserving the existing command semantics and on-board validation.
- Expose a safety-classified command surface: read-only, setup/debug, Adapter, risky/destructive, and diagnostic/internal groups.
- Add request IDs, structured JSON error responses, read-only parameter validation, and handlers for large transfer payloads.
- Add a queued job model for long-running operations such as `FLASH`, `CAPTURE`, and `MASSERASE`, with progress delivered by SSE reconnection and operation ID filtering.
- Make WebUI controls pass their form values instead of calling acceptance-only methods.
- Enable command execution on the network side without duplicating the existing CLI and CDC command behavior where avoidable.
- Mark the prior archived WebUI task statuses as trust-level rather than implementation proof.

## Capabilities

### New Capabilities
- `webui-mcp-api`: JSON-RPC bridge, command discovery, execution routing, structured responses, and error taxonomy.
- `network-command-jobs`: Queued long-running operations, handles, progress events, and cancellation semantics.
- `webui-control-surface`: Browser-facing tool windows and form bindings over the WebUI/MCP command surface.

### Modified Capabilities
- `network-access`: Broaden from passive probe-side RNDIS access to an actual command bridge and WebUI control path, with command handles, progress events, and safety-class enforcement.

## Impact

- Affected firmware: `firmware/src/network_stack.c`, `firmware/src/mcp_endpoint.c`, `firmware/src/programmer/*`, `firmware/src/analyzer/capture.*`, `firmware/src/adapter/*`, `firmware/src/main.c`, and `firmware/src/structs.h`.
- Affected WebUI assets: `firmware/html/index.html`, static asset build conversion, and any generated WinBox tools.
- Affected API/SDK: `sdk/agentla` command assumptions, host UI wiring, supported method names, and error dictionaries.
- Affected OpenSpec: archived webGUI over-marked completion and future task status trust.