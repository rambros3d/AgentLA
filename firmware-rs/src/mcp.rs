//! MCP server (stub for now): defines types, no transport yet.
//! Phase follows proposals only.
pub enum JsonRpcError { ParseError = -32700, InvalidRequest = -32600, MethodNotFound = -32601 }
pub struct Tool { pub id: usize, pub name: &'static str, pub description: &'static str }
pub static TOOLS: [Tool; 4] = [
    Tool { id: 0, name: "status", description: "Device status summary" },
    Tool { id: 1, name: "probe_target", description: "Probe target device" },
    Tool { id: 2, name: "read_pins", description: "Read flex pin states" },
    Tool { id: 3, name: "system_reboot_bootsel", description: "Reboot into BOOTSEL" },
];
