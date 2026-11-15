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

pub static DEFAULT_RESOURCES: [Resource; 7] = [
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
    Resource::new(
        "agentla://skills/mcu-autonomous-testing",
        "MCU Autonomous Testing Playbook",
        Some("Universal closed-loop hardware testing, firmware validation, and HardFault triage guide"),
        "text/markdown",
    ),
    Resource::new(
        "agentla://skills/dev",
        "Autonomous Firmware Developer Specialist",
        Some("Closed-loop firmware synthesis, wiring guidance, flashing, and physical verification"),
        "text/markdown",
    ),
    Resource::new(
        "agentla://skills/debug",
        "Hardware Triage & Debug Specialist",
        Some("Target rail voltage check, CoreSight DPIDR/CPUID discovery, and HardFault triage"),
        "text/markdown",
    ),
    Resource::new(
        "agentla://skills/programmer",
        "Target Flash & Unbrick Specialist",
        Some("Chip catalog lookup, sector-aligned flashing, readback verification, and rescue"),
        "text/markdown",
    ),
    Resource::new(
        "agentla://skills/analyzer",
        "Bus Protocol & Logic Analyzer Specialist",
        Some("Pin Arbiter reservation, high-speed signal capture, protocol decoding, and 120-ohm termination"),
        "text/markdown",
    ),
];

/// An MCP Prompt Argument definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PromptArgument {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub required: bool,
}

impl PromptArgument {
    pub const fn new(
        name: &'static str,
        description: Option<&'static str>,
        required: bool,
    ) -> Self {
        Self {
            name,
            description,
            required,
        }
    }

    pub fn serialize<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
        write!(w, r#"{{"name":"{}""#, self.name)?;
        if let Some(desc) = self.description {
            write!(w, r#","description":"{}""#, desc)?;
        }
        write!(w, r#","required":{}}}"#, self.required)
    }
}

/// An MCP Prompt definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prompt {
    pub name: &'static str,
    pub description: Option<&'static str>,
    pub arguments: &'static [PromptArgument],
}

impl Prompt {
    pub const fn new(
        name: &'static str,
        description: Option<&'static str>,
        arguments: &'static [PromptArgument],
    ) -> Self {
        Self {
            name,
            description,
            arguments,
        }
    }

    pub fn serialize<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
        write!(w, r#"{{"name":"{}""#, self.name)?;
        if let Some(desc) = self.description {
            write!(w, r#","description":"{}""#, desc)?;
        }
        if !self.arguments.is_empty() {
            write!(w, r#","arguments":["#)?;
            for (i, arg) in self.arguments.iter().enumerate() {
                if i > 0 {
                    write!(w, ",")?;
                }
                arg.serialize(w)?;
            }
            write!(w, "]")?;
        }
        write!(w, "}}")
    }
}

pub static DEV_WORKFLOW_ARGS: [PromptArgument; 1] = [
    PromptArgument::new(
        "target",
        Some("Target MCU part number or architecture (e.g. STM32F401, RP2040, CH32V203)"),
        false,
    ),
];

pub static DEBUG_TARGET_ARGS: [PromptArgument; 1] = [
    PromptArgument::new(
        "target",
        Some("Target MCU port or name (e.g. bluepill, ch32v203, default)"),
        false,
    ),
];

pub static PROGRAM_TARGET_ARGS: [PromptArgument; 1] = [
    PromptArgument::new(
        "target",
        Some("Target MCU part number for catalog lookup (e.g. STM32F103, STM32F401, RP2040)"),
        false,
    ),
];

pub static ANALYZE_BUS_ARGS: [PromptArgument; 1] = [
    PromptArgument::new(
        "protocol",
        Some("Protocol to decode (e.g. i2c, spi, uart, can)"),
        false,
    ),
];

pub static DEFAULT_PROMPTS: [Prompt; 4] = [
    Prompt::new(
        "dev_workflow",
        Some("Autonomous closed-loop firmware development, wiring guidance, flashing, and physical signal verification"),
        &DEV_WORKFLOW_ARGS,
    ),
    Prompt::new(
        "debug_target",
        Some("Hardware electrical health check, CoreSight identification, and Cortex-M HardFault register triage"),
        &DEBUG_TARGET_ARGS,
    ),
    Prompt::new(
        "program_target",
        Some("Universal target memory catalog lookup, sector erase, flash programming, and unbrick recovery"),
        &PROGRAM_TARGET_ARGS,
    ),
    Prompt::new(
        "analyze_bus",
        Some("Pin Arbiter reservation, high-speed logic analyzer capture, and protocol decoding"),
        &ANALYZE_BUS_ARGS,
    ),
];

/// The MCP JSON-RPC 2.0 Server.
pub struct McpServer<'a, H = ()> {
    pub server_name: &'a str,
    pub server_version: &'a str,
    pub tools: &'a [Tool],
    pub resources: &'a [Resource],
    pub prompts: &'a [Prompt],
    pub handler: Option<H>,
}

impl McpServer<'static, ()> {
    pub const fn new() -> Self {
        Self {
            server_name: "AgentLA RP2350",
            server_version: "rust-1.0",
            tools: &ALL_TOOLS,
            resources: &DEFAULT_RESOURCES,
            prompts: &DEFAULT_PROMPTS,
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
            prompts: &DEFAULT_PROMPTS,
            handler: Some(handler),
        }
    }

    pub fn set_tools(&mut self, tools: &'a [Tool]) {
        self.tools = tools;
    }

    pub fn set_resources(&mut self, resources: &'a [Resource]) {
        self.resources = resources;
    }

    pub fn set_prompts(&mut self, prompts: &'a [Prompt]) {
        self.prompts = prompts;
    }

    /// Handle a request and return an 8192-byte heapless string response.
    pub fn handle_request(&self, body: &[u8]) -> heapless::String<8192> {
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
                    r#"{{"jsonrpc":"2.0","id":{},"result":{{"protocolVersion":"2025-03-26","serverInfo":{{"name":"{}","version":"{}"}},"capabilities":{{"tools":{{}},"resources":{{}},"prompts":{{}}}},"instructions":"Hardware tools available via tools/call, resources via resources/read, prompts via prompts/get."}}}}"#,
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
                        "agentla://skills/mcu-autonomous-testing" => {
                            let _ = write!(
                                out,
                                r##"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://skills/mcu-autonomous-testing","mimeType":"text/markdown","text":"# Universal MCU Autonomous Testing Playbook\n\nEndpoint: http://10.10.10.10/skills/mcu-autonomous-testing/SKILL.md\n\nPhases:\n1. Auto-Discovery & Health Audit\n2. Pre-Flash Invariant Check & Deployment\n3. Boot Settling (100ms) & Console Handshake\n4. Multi-Protocol Interactive Stimulus\n5. Autonomous HardFault Triage (target_diagnose)"}}]}}}}"##,
                                id_frag
                            );
                        }
                        "agentla://skills/dev" | "agentla://skills/agentla-dev" => {
                            let _ = write!(
                                out,
                                r##"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://skills/dev","mimeType":"text/markdown","text":"# Autonomous Firmware Developer & Verification Specialist (`agentla-dev`)\n\nEndpoint: http://10.10.10.10/skills/agentla-dev/SKILL.md\n\nClosed-Loop Developer Flow:\n1. Requirements & Pinout Consultation (catalog_lookup, pinout mapping)\n2. Pre-Flight Safety & Silicon Detection (status, probe_target, target_diagnose)\n3. Target Firmware Synthesis & Build (host environment compilation)\n4. Target Flash Programming (programmer_flash with sector alignment)\n5. Pin Arbiter Allocation (pins_claim for logic analyzer channels)\n6. Physical Signal Verification (sniff_bus / logic capture, frequency & duty cycle check)\n7. Self-Healing & Diagnostic Loop (automated retry or HardFault triage on failure)\n8. Final Production Lock & Report (pins_release, verified deployment)"}}]}}}}"##,
                                id_frag
                            );
                        }
                        "agentla://skills/debug" | "agentla://skills/agentla-debug" => {
                            let _ = write!(
                                out,
                                r##"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://skills/debug","mimeType":"text/markdown","text":"# Hardware Triage & Debug Specialist (`agentla-debug`)\n\nEndpoint: http://10.10.10.10/skills/agentla-debug/SKILL.md\n\nDiagnostic & Triage Flow:\n1. Electrical Rail & Ground Integrity (1.65V <= Vtarget <= 3.6V check)\n2. Target Health Diagnostic (target_diagnose for rail, current, and SWD state)\n3. CoreSight Debug Port & CPUID Discovery (DPIDR read, CPUID architecture decode)\n4. Access Port (AP) Enumeration (AP scan, ROM table traversal)\n5. Cortex-M HardFault Post-Mortem Triage (CFSR, HFSR, MMFAR, BFAR decode)\n6. Non-Destructive Live Memory Inspection (safe register read, stack frame walk)"}}]}}}}"##,
                                id_frag
                            );
                        }
                        "agentla://skills/programmer" | "agentla://skills/agentla-programmer" => {
                            let _ = write!(
                                out,
                                r##"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://skills/programmer","mimeType":"text/markdown","text":"# Target Flash & Unbrick Specialist (`agentla-programmer`)\n\nEndpoint: http://10.10.10.10/skills/agentla-programmer/SKILL.md\n\nProgramming & Rescue Flow:\n1. Target Catalog & Memory Bounds Binding (catalog_lookup for flash base and sector layout)\n2. Electrical Safety & Target Detection (probe_target / target_diagnose)\n3. Target Mass Erase & Protection Check (inspect read/write protection status)\n4. Chunked Sector Flash Programming (programmer_flash with CRC verification)\n5. Integrity & Readback Verification (programmer_read or CRC-32 match)\n6. Target Rescue & Unbrick Sequences (programmer_unbrick with CTRL-AP mass erase / reset glitch)"}}]}}}}"##,
                                id_frag
                            );
                        }
                        "agentla://skills/analyzer" | "agentla://skills/agentla-analyzer" => {
                            let _ = write!(
                                out,
                                r##"{{"jsonrpc":"2.0","id":{},"result":{{"contents":[{{"uri":"agentla://skills/analyzer","mimeType":"text/markdown","text":"# Bus Protocol & Logic Analyzer Specialist (`agentla-analyzer`)\n\nEndpoint: http://10.10.10.10/skills/agentla-analyzer/SKILL.md\n\nSignal Capture & Analysis Flow:\n1. Pin Arbiter Claim (pins_claim on channels D0..D7)\n2. Logic Capture Configuration (sample rate, threshold, trigger conditions)\n3. Bus Protocol Decoding (sniff_bus for I2C, SPI, UART, CAN-2.0B / CAN-FD)\n4. CAN Bus Termination & Injection Control (120-ohm termination relay control)\n5. Signal Validation & Assertion (timing verification, frame ACK/NACK validation)\n6. Pin Arbiter Release (pins_release when analysis complete)"}}]}}}}"##,
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
            "prompts/list" => {
                let _ = write!(out, r#"{{"jsonrpc":"2.0","id":{},"result":{{"prompts":["#, id_frag);
                for (i, prompt) in self.prompts.iter().enumerate() {
                    if i > 0 {
                        let _ = out.push(',');
                    }
                    let _ = prompt.serialize(out);
                }
                let _ = out.push_str("]}}}");
            }
            "prompts/get" => {
                if let Some(name) = extract_string_arg(text, "name") {
                    match name {
                        "dev_workflow" => {
                            let target = extract_string_arg(text, "target").unwrap_or("target MCU");
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"description":"Autonomous closed-loop firmware development workflow","messages":[{{"role":"user","content":{{"type":"text","text":"Autonomous firmware development for {}. Follow 8-phase closed-loop playbook: (1) catalog_lookup, (2) user wiring guidance, (3) target_diagnose rail safety, (4) compile, (5) programmer_flash, (6) pins_claim, (7) physical signal verification via logic analyzer, (8) pins_release. Reference: http://10.10.10.10/skills/agentla-dev/SKILL.md"}}]}}}}"#,
                                id_frag, target
                            );
                        }
                        "debug_target" => {
                            let target = extract_string_arg(text, "target").unwrap_or("connected target");
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"description":"Hardware electrical triage and HardFault post-mortem triage","messages":[{{"role":"user","content":{{"type":"text","text":"Hardware triage for {}. Follow debug playbook: (1) status electrical rail check (1.65V-3.6V), (2) target_diagnose CoreSight discovery, (3) CFSR/HFSR/MMFAR/BFAR fault register decode, (4) stacked frame analysis. Reference: http://10.10.10.10/skills/agentla-debug/SKILL.md"}}]}}}}"#,
                                id_frag, target
                            );
                        }
                        "program_target" => {
                            let target = extract_string_arg(text, "target").unwrap_or("target MCU");
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"description":"Target flash programming and unbrick sequence","messages":[{{"role":"user","content":{{"type":"text","text":"Target programming for {}. Follow programming playbook: (1) catalog_lookup for flash boundaries, (2) probe_target connection test, (3) protection check / programmer_unbrick if locked, (4) sector-aligned programmer_flash, (5) CRC readback verification. Reference: http://10.10.10.10/skills/agentla-programmer/SKILL.md"}}]}}}}"#,
                                id_frag, target
                            );
                        }
                        "analyze_bus" => {
                            let protocol = extract_string_arg(text, "protocol").unwrap_or("auto");
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"result":{{"description":"Bus protocol capture and logic analyzer decoding","messages":[{{"role":"user","content":{{"type":"text","text":"Protocol analysis for {}. Follow analyzer playbook: (1) pins_claim probe channels D0..D7, (2) set 120-ohm CAN termination relay if needed, (3) configure trigger & sample rate, (4) sniff_bus protocol decode, (5) timing & checksum validation, (6) pins_release. Reference: http://10.10.10.10/skills/agentla-analyzer/SKILL.md"}}]}}}}"#,
                                id_frag, protocol
                            );
                        }
                        _ => {
                            let _ = write!(
                                out,
                                r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32602,"message":"prompt not found"}}}}"#,
                                id_frag
                            );
                        }
                    }
                } else {
                    let _ = write!(
                        out,
                        r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":-32602,"message":"missing name parameter"}}}}"#,
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
