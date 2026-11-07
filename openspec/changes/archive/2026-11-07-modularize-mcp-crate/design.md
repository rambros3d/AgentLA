## Context

`firmware-rs` provides an embedded MCP endpoint at `/api/v1/mcp` answering JSON-RPC 2.0 calls.
This design moves JSON-RPC framing, tool registration, parameter extraction, and error formatting into `crates/agentla-mcp`.

## Goals / Non-Goals

**Goals:**
- Extract `JsonRpcRequest`, `JsonRpcResponse`, `Tool`, and `ToolHandler` into `crates/agentla-mcp`.
- Ensure zero dynamic allocation using `heapless::String` buffers.
- Host unit tests verifying JSON serialization and parameter extraction on `x86_64`.

**Non-Goals:**
- Managing HTTP socket lifecycles.
