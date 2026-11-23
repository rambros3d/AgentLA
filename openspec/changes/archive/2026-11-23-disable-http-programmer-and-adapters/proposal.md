# Proposal

## Why

Executing hardware programming and bus bridging over embedded HTTP REST creates resource contention on the Embassy executor, causes timing jitter and false SWD parity faults, lacks hardware flow control, and conflicts with simultaneous native USB interfaces (CMSIS-DAP, ST-Link, i2c-tiny-usb, FTDI MPSSE, and AgentLA I3C Bulk). Temporarily disabling HTTP programmer and bus adapter routes eliminates on-chip contention and aligns firmware, web UI, and AI agents around direct USB hardware endpoints.

## What Changes

- **Temporarily Disable HTTP Programmer Routes**: All `/api/v1/programmer/*` endpoints remain registered in the HTTP router but return a structured warning payload directing clients and agents to use native USB debug interfaces (CMSIS-DAP, ST-Link, or WCH-Link).
- **Temporarily Disable HTTP Bus Adapter Routes**: All `/api/v1/bridge/*` and `/api/v1/bus/*` (I2C, I3C, SPI, UART, CAN) endpoints remain registered in the HTTP router but return a structured warning payload directing clients and agents to use direct USB adapters (`i2c-tiny-usb`, `ftdi-mpsse`, `agentla-binary-bulk`, `gs_usb`, `cdc-acm`).
- **Update MCP Agent Surface**: Update `agentla-mcp` tools, resources, and prompt definitions (`agentla://skills/programmer`, `agentla://skills/dev`, `program_target`, `dev_workflow`) so autonomous agents recognize that flashing, reading target memory, and high-speed bus transfers must be driven via direct host USB adapters rather than HTTP routes.
- **MCP Tool Count and Health Alignment**: Ensure `/mcp/health` and MCP catalog reports consistent tool counts (58 tools) across crates and tests.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `rest-target-programmer`: HTTP target programmer endpoints return structured warning payloads directing clients to direct USB debug probes.
- `rest-bus-bridges`: HTTP bus adapter endpoints return structured warning payloads directing clients to native USB adapter devices.
- `agentla-programmer`: Programmer agent skill and MCP prompt sequences specify direct USB flashing and debugging workflows.

## Impact

- `firmware-rs/src/rest.rs`: Introduces `HTTP_PROGRAMMER_ENABLED: bool = false` and `HTTP_ADAPTERS_ENABLED: bool = false` guards with JSON warning bodies; updates MCP health reporting tool count to 58.
- `firmware-rs/src/mcp.rs`: Returns USB transport warnings for `programmer_flash`, `programmer_unbrick`, and `programmer_read`.
- `crates/agentla-mcp/src/server.rs`: Updates skill texts and prompts to direct agents to direct USB tooling (`probe-rs`, `pyocd`, `openocd`, host SDKs).
- Host Tools and WebUI: Direct USB connections are utilized for SWD programming, I2C, SPI, I3C, and CAN transfers.
