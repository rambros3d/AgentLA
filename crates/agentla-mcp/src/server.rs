//! MCP JSON-RPC 2.0 Server dispatcher.

use core::fmt::Write as _;
use crate::protocol::{escape_json, extract_id, extract_method, extract_string_arg, extract_tool_name, Id};
use crate::tools::{ALL_TOOLS, Tool, ToolHandler};

/// An MCP Resource definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resource {
    pub uri: &'static str,
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub mime_type: &'static str,
}

impl Resource {
    pub const fn new(
        uri: &'static str,
        name: &'static str,
        description: Option<&'static str>,
        mime_type: &'static str,
    ) -> Self {
        Self {
            uri,
            name,
            description,
            mime_type,
        }
    }

    pub fn serialize<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
        write!(w, r#"{{"uri":"{}","name":"{}""#, self.uri, self.name)?;
        if let Some(desc) = self.description {
            write!(w, r#","description":"{}""#, desc)?;
        }
        write!(w, r#","mimeType":"{}"}}"#, self.mime_type)
    }
}

pub static DEFAULT_RESOURCES: [Resource; 2] = [
    Resource::new(
        "agentla://info",
        "AgentLA System Info",
        Some("Probe status, firmware version, and target configuration"),
        "application/json",
    ),
    Resource::new(
        "agentla://pinout",
        "AgentLA Pin Allocation",
        Some("GPIO pin assignments and subsystem allocations"),
        "application/json",
    ),
];

/// The MCP JSON-RPC 2.0 Server.
pub struct McpServer<'a, H = ()> {
    pub server_name: &'a str,
    pub server_version: &'a str,
    pub tools: &'a [Tool],
    pub resources: &'a [Resource],
    pub handler: Option<H>,
}

impl McpServer<'static, ()> {
    pub const fn new() -> Self {
        Self {
            server_name: "AgentLA RP2350",
            server_version: "rust-1.0",
            tools: &ALL_TOOLS,
            resources: &DEFAULT_RESOURCES,
            handler: None,
        }
    }
}

impl<'a, H: ToolHandler> McpServer<'a, H> {
    pub const fn with_handler(handler: H) -> Self {
        Self {
            server_name: "AgentLA RP2350",
            server_version: "rust-1.0",
            tools: &ALL_TOOLS,
            resources: &DEFAULT_RESOURCES,
            handler: Some(handler),
        }
    }

    pub fn set_tools(&mut self, tools: &'a [Tool]) {
        self.tools = tools;
    }

    pub fn set_resources(&mut self, resources: &'a [Resource]) {
        self.resources = resources;
    }

    /// Handle a request and return a 3072-byte heapless string response.
    pub fn handle_request(&self, body: &[u8]) -> heapless::String<3072> {
        let mut out = heapless::String::new();
        self.handle_request_into(body, &mut out);
        out
    }

    /// Handle a request into any heapless string buffer.
    pub fn handle_request_into<const N: usize>(&self, body: &[u8], out: &mut heapless::String<N>) {
        let text = core::str::from_utf8(body).unwrap_or("{}");
        let method = extract_method(text);
        let id = extract_id(text);
        let id_frag = id.unwrap_or(Id::Null);

        match method {
            "initialize" => {
                let _ = write!(
                    out,
                    r#"{{"jsonrpc":"2.0","id":{},"result":{{"protocolVersion":"2025-03-26","serverInfo":{{"name":"{}","version":"{}"}},"capabilities":{{"tools":{{}},"resources":{{}}}},"instructions":"Hardware tools available via tools/call."}}}}"#,
                    id_frag, self.server_name, self.server_version
                );
            }
            "notifications/initialized" => {
                out.clear();
            }
            "ping" => {
                let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{}}}}"#, id_frag);
            }
            "tools/list" => {
                let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"tools":["#, id_frag);
                for (i, tool) in self.tools.iter().enumerate() {
                    if i > 0 {
                        let _ = out.push(',');
                    }
                    let _ = tool.serialize(out);
                }
                let _ = out.push_str("]}}}");
            }
            "tools/call" => {
                let name = extract_tool_name(text).unwrap_or_default();
                if let Some(ref handler) = self.handler {
                    let mut tool_out = heapless::String::<1536>::new();
                    let is_error = handler.handle_tool(name.as_str(), text, &mut tool_out);
                    let mut escaped = heapless::String::<2048>::new();
                    escape_json(tool_out.as_str(), &mut escaped);
                    let _ = write!(
                        out,
                        r#"{{"jsonrpc":"2.0","id":{},"result":{{"content":[{{"type":"text","text":"{}"}}],"isError":{}}}}}"#,
                        id_frag,
                        escaped.as_str(),
                        is_error
                    );
                } else {
                    let _ = write!(
                        out,
                        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32603,"message":"tool handler not registered"}}}}"#,
                        id_frag
                    );
                }
            }
            "resources/list" => {
                let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"resources":["#, id_frag);
                for (i, res) in self.resources.iter().enumerate() {
                    if i > 0 {
                        let _ = out.push(',');
                    }
                    let _ = res.serialize(out);
                }
                let _ = out.push_str("]}}}");
            }
            "resources/read" => {
                if let Some(uri) = extract_string_arg(text, "uri") {
                    match uri {
                        "agentla://info" => {
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://info","mimeType":"application/json","text":"{{\"name\":\"{}\",\"version\":\"{}\",\"status\":\"ready\"}}"}}]}}}}"#,
                                id_frag, self.server_name, self.server_version
                            );
                        }
                        "agentla://pinout" => {
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://pinout","mimeType":"application/json","text":"{{\"reserved\":\"GPIO0-GPIO9\",\"flexible\":\"GPIO10-GPIO29\"}}"}}]}}}}"#,
                                id_frag
                            );
                        }
                        _ => {
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32602,"message":"resource not found"}}}}"#,
                                id_frag
                            );
                        }
                    }
                } else {
                    let _ = write!(
                        out,
                        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32602,"message":"missing uri parameter"}}}}"#,
                        id_frag
                    );
                }
            }
            _ => {
                let _ = write!(
                    out,
                    r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32601,"message":"method not found"}}}}"#,
                    id_frag
                );
            }
        }
    }
}
