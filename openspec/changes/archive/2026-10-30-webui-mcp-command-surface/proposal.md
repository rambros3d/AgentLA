## Why

AgentLA now has a firmware command bridge, but the WebUI still does not expose the full firmware command surface as accessible, discoverable, and safely interactive operations. This change closes the gap between available firmware capabilities and WebUI availability.

## What Changes

- Define command-level API behavior over `/mcp`.
- Expose command families such as `IDENTIFY`, `STATUS`, `CONFIGURE`, `CAPTURE`, `MEASURE`, `FLASH`, `DEBUG`, `SWD`, `SWIO`, `RVSWD`, `PROGBUF`, `MASSERASE`, `PIN*`, and `ADC`.
- Add parameter mapping, validation, and discovery responses.
- Add operation handles for long operations and progress events.
- Add WebUI command forms for at least one complete workflow as the first migration pattern.
- Reconcile inflated archived WebUI task statuses.
- Mark BREAKING changes for UI and API method expectations where required.

## Capabilities

### New Capabilities
- `webui-mcp-command-surface`: Command discovery, command execution APIs, result envelopes, and safety classification as a public Web interface.
- `webui-workflow-bridge`: Browser-visible command forms and parameter handling for non-CLI workflows.
- `network-command-fidelity`: Accurate status and operation progress without falsely exposing Core 1 execution.

### Modified Capabilities
- `network-access`: Network requests and results become command-capable rather than static page behavior.

## Impact

- Affected firmware network and dispatch code.
- Affected MCP/WebUI assets and browser-side behavior.
- Affected OpenSpec statuses and archived webGUI assumptions.