## 1. Implement MCP Crate (`agentla-mcp`)

- [x] 1.1 Implement JSON-RPC 2.0 types and request/response serializers
- [x] 1.2 Implement MCP tool definitions and schema catalog
- [x] 1.3 Add host unit tests (`tests/mcp_rpc.rs`) on x86_64

## 2. Firmware Integration (`firmware-rs`)

- [x] 2.1 Update `firmware-rs/Cargo.toml` to depend on `agentla-mcp`
- [x] 2.2 Refactor `firmware-rs/src/mcp.rs` to delegate schema generation and dispatch to `agentla-mcp`

## 3. Verification

- [x] 3.1 Run `cargo test --package agentla-mcp` on host x86_64
- [x] 3.2 Build bare-metal firmware `cargo build --package firmware-rs --target thumbv8m.main-none-eabihf --release`
- [x] 3.3 Validate zero regressions
