## Why

The Model Context Protocol (MCP) server in `firmware-rs` (`mcp.rs`) exposes AgentLA hardware tools, bus sniffers, and target diagnostic routines to AI agents over JSON-RPC 2.0. Currently, the tool definitions, parameter schemas, and request dispatch logic are coupled with HTTP route handlers.

Extracting this into `crates/agentla-mcp` establishes a transport-independent MCP JSON-RPC 2.0 engine runnable over HTTP, WebSockets, USB Serial, or UART, and enables unit testing tool serialization and parsing on the desktop.

## What Changes

- **Add `crates/agentla-mcp`**:
  - Implements JSON-RPC 2.0 request parsing and response formatting (`initialize`, `tools/list`, `tools/call`, `resources/list`, `resources/read`).
  - Implements static schemas for AgentLA tools (`inspect_firmware`, `sniff_bus`, `target_diagnose`).
  - Native `#![no_std]` using `heapless` strings without dynamic memory allocation.
  - Comprehensive host unit tests on `x86_64`.
- **Refactor `firmware-rs`**:
  - Updates `firmware-rs/Cargo.toml` to depend on `agentla-mcp`.
  - Refactors `firmware-rs/src/mcp.rs` to delegate JSON-RPC schema generation and dispatching to `agentla-mcp`.

## Capabilities

### New Capabilities
- `domain-crate-mcp`: Embedded Model Context Protocol JSON-RPC 2.0 schema and tool dispatcher.

### Modified Capabilities
<!-- None: External MCP client tool interactions remain 100% compliant. -->

## Impact

- **Codebase**: `firmware-rs/src/mcp.rs` delegates to `agentla-mcp`.
- **Cargo Workspace**: `crates/agentla-mcp` implemented with unit tests.
