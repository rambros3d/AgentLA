//! Tool definitions, schemas, and dispatch traits for AgentLA MCP server.

/// Definition of an MCP tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: &'static str,
}

impl Tool {
    pub const fn new(name: &'static str, description: &'static str, input_schema: &'static str) -> Self {
        Self {
            name,
            description,
            input_schema,
        }
    }

    pub fn serialize<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
        write!(
            w,
            r#"{{"name":"{}","description":"{}","inputSchema":{}}}"#,
            self.name, self.description, self.input_schema
        )
    }
}

pub const TOOL_STATUS: Tool = Tool::new(
    "status",
    "Probe status, clocks, network",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROBE_TARGET: Tool = Tool::new(
    "probe_target",
    "Detect target family on port 1",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_READ_PINS: Tool = Tool::new(
    "read_pins",
    "Read pins 0-29 sample",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_SYSTEM_FIRMWARE_STATUS: Tool = Tool::new(
    "system_firmware_status",
    "Query A/B firmware OTA slot status and boot state",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROFILE_STATUS: Tool = Tool::new(
    "profile_status",
    "Active profile slot",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_SERVO_LIST: Tool = Tool::new(
    "servo_list",
    "List active servo channels",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_LOGIC_ANALYZER_ARM: Tool = Tool::new(
    "logic_analyzer_arm",
    "Arm logic analyzer for capture",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_LOGIC_ANALYZER_START: Tool = Tool::new(
    "logic_analyzer_start",
    "Trigger logic capture",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_LOGIC_ANALYZER_STATUS: Tool = Tool::new(
    "logic_analyzer_status",
    "Capture state and sample count",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_LOGIC_ANALYZER_DATA: Tool = Tool::new(
    "logic_analyzer_data",
    "Inspect sample words",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROGRAMMER_READ: Tool = Tool::new(
    "programmer_read",
    "Read target flash memory",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROGRAMMER_RESET: Tool = Tool::new(
    "programmer_reset",
    "Target reset",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_I2C_SCAN: Tool = Tool::new(
    "i2c_scan",
    "Scan I2C bus for target devices",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_WS2812_TRANSMIT: Tool = Tool::new(
    "ws2812_transmit",
    "Transmit LED pattern",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_TARGET_DIAGNOSE: Tool = Tool::new(
    "target_diagnose",
    "One-shot diagnostic inspecting target electrical rail voltage, pin continuity/pull-up state, CoreSight DPIDR/CPUID, and CPU HardFault register root-cause analysis",
    r#"{"type":"object","properties":{"port":{"type":"string","description":"Target port (bluepill, ch32v203, ch32v003)"}}}"#,
);

pub const TOOL_INSPECT_FIRMWARE: Tool = Tool::new(
    "inspect_firmware",
    "Inspect vector table, validate Stack Pointer against SRAM bounds, and verify ARM Thumb mode invariant",
    r#"{"type":"object","properties":{"address":{"type":"integer","description":"Flash base address (default 0x08000000)"},"target":{"type":"string","description":"Target chip family name to lookup LittleFS FLM algorithm"}}}"#,
);

pub const TOOL_SNIFF_BUS: Tool = Tool::new(
    "sniff_bus",
    "Autonomous bus transaction sniffer for I2C and SPI",
    r#"{"type":"object","properties":{"protocol":{"type":"string","enum":["i2c","spi"],"description":"Bus protocol to sniff"},"duration_ms":{"type":"integer","description":"Sniff duration in milliseconds (50 - 1000)"}},"required":["protocol"]}"#,
);

pub const TOOL_PROGRAMMER_UNBRICK: Tool = Tool::new(
    "programmer_unbrick",
    "Unbrick / mass-erase protected target microcontroller across STM32 RDP, Nordic APPROTECT, NXP MDM-AP, and Microchip SAMD",
    r#"{"type":"object","properties":{"target":{"type":"string","description":"Target family or commercial part number"}}}"#,
);

pub const TOOL_CATALOG_LOOKUP: Tool = Tool::new(
    "catalog_lookup",
    "Query universal target catalog by part number or DevID to inspect flash/RAM size and direct engine",
    r#"{"type":"object","properties":{"query":{"type":"string","description":"Part number prefix (e.g. STM32F401, nRF52840, CH32V003)"}},"required":["query"]}"#,
);

/// Three composite AI Agent hardware tools.
pub static COMPOSITE_TOOLS: [Tool; 3] = [
    TOOL_TARGET_DIAGNOSE,
    TOOL_INSPECT_FIRMWARE,
    TOOL_SNIFF_BUS,
];

/// Full catalog of AgentLA tools (16 standard + 3 composite tools).
pub static ALL_TOOLS: [Tool; 19] = [
    TOOL_STATUS,
    TOOL_PROBE_TARGET,
    TOOL_READ_PINS,
    TOOL_SYSTEM_FIRMWARE_STATUS,
    TOOL_PROFILE_STATUS,
    TOOL_SERVO_LIST,
    TOOL_LOGIC_ANALYZER_ARM,
    TOOL_LOGIC_ANALYZER_START,
    TOOL_LOGIC_ANALYZER_STATUS,
    TOOL_LOGIC_ANALYZER_DATA,
    TOOL_PROGRAMMER_READ,
    TOOL_PROGRAMMER_RESET,
    TOOL_PROGRAMMER_UNBRICK,
    TOOL_CATALOG_LOOKUP,
    TOOL_I2C_SCAN,
    TOOL_WS2812_TRANSMIT,
    TOOL_TARGET_DIAGNOSE,
    TOOL_INSPECT_FIRMWARE,
    TOOL_SNIFF_BUS,
];

/// Trait for dispatching tool calls to the hardware engine.
pub trait ToolHandler {
    /// Dispatches a tool by name and raw request body.
    ///
    /// Writes output into `out` and returns `true` if the tool returned an error (`isError: true`),
    /// or `false` for success.
    fn handle_tool(&self, name: &str, body: &str, out: &mut heapless::String<1536>) -> bool;
}

impl ToolHandler for () {
    fn handle_tool(&self, _name: &str, _body: &str, out: &mut heapless::String<1536>) -> bool {
        let _ = out.push_str("tool handler not registered");
        true
    }
}

impl<F> ToolHandler for F
where
    F: Fn(&str, &str, &mut heapless::String<1536>) -> bool,
{
    fn handle_tool(&self, name: &str, body: &str, out: &mut heapless::String<1536>) -> bool {
        self(name, body, out)
    }
}
