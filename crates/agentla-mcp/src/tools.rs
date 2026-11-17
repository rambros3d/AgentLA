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

pub const TOOL_SERVO_ATTACH: Tool = Tool::new(
    "servo_attach",
    "Attach a GPIO pin as a 50 Hz PWM servo channel with optional pulse limits and continuous mode",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"min_us":{"type":"integer","description":"Minimum pulse width in microseconds (default 544)"},"max_us":{"type":"integer","description":"Maximum pulse width in microseconds (default 2400)"},"continuous":{"type":"boolean","description":"Continuous 360-degree rotation mode (default false)"}},"required":["pin"]}"#,
);

pub const TOOL_SERVO_WRITE_ANGLE: Tool = Tool::new(
    "servo_write_angle",
    "Command target angle (0 to 180 degrees) or continuous rotation throttle (-100% to +100%)",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"angle":{"type":"integer","description":"Target angle (0-180) or continuous throttle (-100 to 100)"}},"required":["pin","angle"]}"#,
);

pub const TOOL_SERVO_WRITE_US: Tool = Tool::new(
    "servo_write_us",
    "Command direct pulse duration in microseconds (clamped to channel limits)",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"pulse_us":{"type":"integer","description":"Pulse duration in microseconds"}},"required":["pin","pulse_us"]}"#,
);

pub const TOOL_SERVO_DETACH: Tool = Tool::new(
    "servo_detach",
    "Stop PWM generation and release GPIO pin back to pin arbiter",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"}},"required":["pin"]}"#,
);

pub const TOOL_LOGIC_ANALYZER_ARM: Tool = Tool::new(
    "logic_analyzer_arm",
    "Arm logic analyzer for capture",
    r#"{"type":"object","properties":{"samples":{"type":"integer","description":"Sample count (default 1024)"},"sample_rate_hz":{"type":"integer","description":"Sample rate in Hz (default 1000000)"}}}"#,
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
    r#"{"type":"object","properties":{"address":{"type":"integer","description":"Target memory address (e.g. 0x08000000)"},"length":{"type":"integer","description":"Number of bytes to read (1-256, default 16)"}}}"#,
);

pub const TOOL_PROGRAMMER_FLASH: Tool = Tool::new(
    "programmer_flash",
    "Write hex-encoded byte chunk to target flash memory with verification",
    r#"{"type":"object","properties":{"address":{"type":"integer","description":"Flash base address (e.g. 0x08000000)"},"hex_data":{"type":"string","description":"Hex-encoded binary bytes to write"}},"required":["address","hex_data"]}"#,
);

pub const TOOL_PROGRAMMER_RESET: Tool = Tool::new(
    "programmer_reset",
    "Target reset",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_I2C_SCAN: Tool = Tool::new(
    "i2c_scan",
    "Scan I2C bus for target devices",
    r#"{"type":"object","properties":{"sda_pin":{"type":"integer","description":"SDA GPIO pin (default 16)"},"scl_pin":{"type":"integer","description":"SCL GPIO pin (default 17)"}}}"#,
);

pub const TOOL_CAN_STATUS: Tool = Tool::new(
    "can_status",
    "Query CAN controller state, active bitrate, error counters, and 120-ohm termination relay",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_CAN_SEND: Tool = Tool::new(
    "can_send",
    "Transmit a CAN frame (standard 11-bit or extended 29-bit)",
    r#"{"type":"object","properties":{"id":{"type":"integer","description":"CAN identifier"},"data_hex":{"type":"string","description":"Hex payload bytes (0 to 8 bytes)"},"extended":{"type":"boolean","description":"True if 29-bit CAN ID"},"rtr":{"type":"boolean","description":"Remote transmission request"}},"required":["id","data_hex"]}"#,
);

pub const TOOL_CAN_SNIFF: Tool = Tool::new(
    "can_sniff",
    "Sniff CAN traffic from hardware RX buffer for bounded interval",
    r#"{"type":"object","properties":{"duration_ms":{"type":"integer","description":"Duration in ms (default 200)"},"max_frames":{"type":"integer","description":"Max frames to capture (default 10)"}}}"#,
);

pub const TOOL_CAN_SET_TERMINATION: Tool = Tool::new(
    "can_set_termination",
    "Enable or disable physical 120-ohm bus termination relay",
    r#"{"type":"object","properties":{"enabled":{"type":"boolean","description":"Enable 120-ohm termination"}},"required":["enabled"]}"#,
);

pub const TOOL_SPI_TRANSFER: Tool = Tool::new(
    "spi_transfer",
    "Execute full-duplex SPI master byte exchange",
    r#"{"type":"object","properties":{"mosi_hex":{"type":"string","description":"Hex bytes to transmit"},"cs_pin":{"type":"integer","description":"Chip select GPIO pin (default 17)"},"speed_khz":{"type":"integer","description":"SPI clock speed in kHz (default 1000)"}},"required":["mosi_hex"]}"#,
);

pub const TOOL_UART_COMMAND: Tool = Tool::new(
    "uart_command",
    "Send command string to target UART console and wait for response",
    r#"{"type":"object","properties":{"tx_data":{"type":"string","description":"String command to transmit"},"baud":{"type":"integer","description":"Baud rate (default 115200)"},"timeout_ms":{"type":"integer","description":"Response timeout in ms (default 500)"}},"required":["tx_data"]}"#,
);

pub const TOOL_PINS_STATUS: Tool = Tool::new(
    "pins_status",
    "Query all 30 GPIO pin allocations, driver ownership, and rail alarms",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PINS_CLAIM: Tool = Tool::new(
    "pins_claim",
    "Atomically claim GPIO pin ownership for a driver in Pin Arbiter",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"driver":{"type":"string","description":"Driver name (uart, i2c, spi, swd, pwm, can, servo)"}},"required":["pin","driver"]}"#,
);

pub const TOOL_PINS_RELEASE: Tool = Tool::new(
    "pins_release",
    "Release GPIO pin back to safe High-Z state in Pin Arbiter",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"}},"required":["pin"]}"#,
);

pub const TOOL_WS2812_TRANSMIT: Tool = Tool::new(
    "ws2812_transmit",
    "Transmit LED pattern (legacy stub, use neopixel_transmit)",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_NEOPIXEL_FILL: Tool = Tool::new(
    "neopixel_fill",
    "Fill addressable LED pixels with a uniform color",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"color":{"type":"string","description":"Color as hex string (#RRGGBB) or named color"},"count":{"type":"integer","description":"Number of pixels to fill (1-64)"}},"required":["color"]}"#,
);

pub const TOOL_NEOPIXEL_SET: Tool = Tool::new(
    "neopixel_set",
    "Set a single addressable LED pixel color by index",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"index":{"type":"integer","description":"Pixel index (0-based)"},"color":{"type":"string","description":"Color as hex string (#RRGGBB) or named color"}},"required":["index","color"]}"#,
);

pub const TOOL_NEOPIXEL_CLEAR: Tool = Tool::new(
    "neopixel_clear",
    "Blank / power-off all addressable LED pixels",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"count":{"type":"integer","description":"Number of pixels to clear (default 64)"}}}"#,
);

pub const TOOL_NEOPIXEL_PATTERN: Tool = Tool::new(
    "neopixel_pattern",
    "Drive procedural LED pattern generator (rainbow, cylon, fire, meteor, pulse, plasma)",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"pattern":{"type":"string","description":"Pattern name (rainbow, cylon, fire, meteor, pulse, plasma, chase, solid)"},"count":{"type":"integer","description":"Number of pixels (default 16)"},"speed":{"type":"integer","description":"Speed percentage (1-100)"}},"required":["pattern"]}"#,
);

pub const TOOL_NEOPIXEL_ANALYZE: Tool = Tool::new(
    "neopixel_analyze",
    "Measure WS2812 signal timing margins and protocol compliance",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"}},"required":["pin"]}"#,
);

pub const TOOL_NEOPIXEL_TRANSMIT: Tool = Tool::new(
    "neopixel_transmit",
    "Transmit an arbitrary array of colors to addressable LED strip",
    r#"{"type":"object","properties":{"pin":{"type":"integer","description":"GPIO pin index (0-29)"},"colors":{"type":"array","items":{"type":"string"},"description":"Array of hex color strings (#RRGGBB)"}},"required":["colors"]}"#,
);

pub const TOOL_TARGET_DIAGNOSE: Tool = Tool::new(
    "target_diagnose",
    "Diagnostic inspecting rail voltage, continuity, CoreSight DPIDR/CPUID, and fault registers",
    r#"{"type":"object","properties":{"port":{"type":"string","description":"Target port (bluepill, ch32v203, ch32v003)"}}}"#,
);

pub const TOOL_INSPECT_FIRMWARE: Tool = Tool::new(
    "inspect_firmware",
    "Inspect vector table, validate Stack Pointer against SRAM bounds, and verify Thumb mode",
    r#"{"type":"object","properties":{"address":{"type":"integer","description":"Flash base address (default 0x08000000)"},"target":{"type":"string","description":"Target chip family name to lookup LittleFS FLM algorithm"}}}"#,
);

pub const TOOL_SNIFF_BUS: Tool = Tool::new(
    "sniff_bus",
    "Autonomous bus transaction sniffer for I2C and SPI",
    r#"{"type":"object","properties":{"protocol":{"type":"string","enum":["i2c","spi"],"description":"Bus protocol to sniff"},"duration_ms":{"type":"integer","description":"Sniff duration in milliseconds (50 - 1000)"}},"required":["protocol"]}"#,
);

pub const TOOL_PROGRAMMER_UNBRICK: Tool = Tool::new(
    "programmer_unbrick",
    "Unbrick / mass-erase protected target across STM32 RDP, Nordic APPROTECT, NXP, Microchip",
    r#"{"type":"object","properties":{"target":{"type":"string","description":"Target family or commercial part number"}}}"#,
);

pub const TOOL_CATALOG_LOOKUP: Tool = Tool::new(
    "catalog_lookup",
    "Query universal target catalog by part number or DevID to inspect flash/RAM size",
    r#"{"type":"object","properties":{"query":{"type":"string","description":"Part number prefix (e.g. STM32F401, nRF52840, CH32V003)"}},"required":["query"]}"#,
);

pub const TOOL_APP_LIST: Tool = Tool::new(
    "app_list",
    "List installed LittleFS applications and storage capacity",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_APP_INSTALL: Tool = Tool::new(
    "app_install",
    "Install application from catalog into LittleFS",
    r#"{"type":"object","properties":{"name":{"type":"string","description":"Catalog ID (e.g. barefoc, servo)"}},"required":["name"]}"#,
);

pub const TOOL_POWER_CONTROL: Tool = Tool::new(
    "power_control",
    "Control target power rail switch and output voltage",
    r#"{"type":"object","properties":{"enabled":{"type":"boolean","description":"Enable target power rail"},"voltage_mv":{"type":"integer","description":"Target voltage in mV"}},"required":["enabled"]}"#,
);

pub const TOOL_POWER_CYCLE: Tool = Tool::new(
    "power_cycle",
    "Pulse target power rail off and on to reboot target",
    r#"{"type":"object","properties":{"delay_ms":{"type":"integer","description":"Power-off duration in ms (default 100)"}}}"#,
);

pub const TOOL_EMERGENCY_SAFE_STATE: Tool = Tool::new(
    "emergency_safe_state",
    "Transition all pins to High-Z and cut target power",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROFILE_LIST: Tool = Tool::new(
    "profile_list",
    "Enumerate hardware profile slots (Slot 0 Safe State, Slots 1-7)",
    r#"{"type":"object","properties":{}}"#,
);

pub const TOOL_PROFILE_APPLY: Tool = Tool::new(
    "profile_apply",
    "Apply hardware profile slot (0-7) or ID",
    r#"{"type":"object","properties":{"slot":{"type":"integer","description":"Slot index (0-7)"},"id":{"type":"string","description":"Profile ID"}}}"#,
);

pub const TOOL_SWO_READ: Tool = Tool::new(
    "swo_read",
    "Drain captured SWO/ITM trace buffer bytes",
    r#"{"type":"object","properties":{"clear":{"type":"boolean","description":"Clear buffer after read (default true)"}}}"#,
);

pub const TOOL_SENSOR_SCAN: Tool = Tool::new(
    "sensor_scan",
    "Scan I2C or SPI bus and return detected device addresses with identified sensor profiles",
    r#"{"type":"object","properties":{"bus":{"type":"string","description":"Bus type: i2c (default) or spi"},"speed_khz":{"type":"integer","description":"Bus clock speed in kHz"},"cs_pin":{"type":"integer","description":"SPI Chip Select GPIO pin (default 17)"}}}"#,
);

pub const TOOL_SENSOR_READ_REG: Tool = Tool::new(
    "sensor_read_reg",
    "Read bytes from sensor register via I2C repeated-start or SPI transfer",
    r#"{"type":"object","properties":{"addr":{"type":"integer","description":"I2C 7-bit device address or SPI dummy address"},"reg":{"type":"integer","description":"Starting register address"},"len":{"type":"integer","description":"Number of bytes to read (1-32, default 1)"},"bus":{"type":"string","description":"Bus type: i2c (default) or spi"},"cs_pin":{"type":"integer","description":"SPI Chip Select pin"}},"required":["addr","reg"]}"#,
);

pub const TOOL_SENSOR_WRITE_REG: Tool = Tool::new(
    "sensor_write_reg",
    "Write one or more bytes to sensor register via I2C or SPI",
    r#"{"type":"object","properties":{"addr":{"type":"integer","description":"I2C 7-bit device address or SPI dummy address"},"reg":{"type":"integer","description":"Target register address"},"data":{"type":"string","description":"Hex-encoded bytes or decimal value to write"},"bus":{"type":"string","description":"Bus type: i2c (default) or spi"},"cs_pin":{"type":"integer","description":"SPI Chip Select pin"}},"required":["addr","reg","data"]}"#,
);

pub const TOOL_SENSOR_POLL: Tool = Tool::new(
    "sensor_poll",
    "Batch poll telemetry registers from recognized sensor (MPU6050, TMAG5273, BME280, AS5600, INA219) and return engineering units",
    r#"{"type":"object","properties":{"addr":{"type":"integer","description":"I2C 7-bit device address"},"profile":{"type":"string","description":"Sensor profile (mpu6050, tmag5273, bme280, as5600, ina219, auto)"},"registers":{"type":"array","items":{"type":"integer"},"description":"Explicit register addresses to poll if profile not specified"}},"required":["addr"]}"#,
);

/// Three composite AI Agent hardware tools.
pub static COMPOSITE_TOOLS: [Tool; 3] = [
    TOOL_TARGET_DIAGNOSE,
    TOOL_INSPECT_FIRMWARE,
    TOOL_SNIFF_BUS,
];

/// Full catalog of AgentLA tools (46 standard + 3 composite tools = 49 total).
pub static ALL_TOOLS: [Tool; 49] = [
    TOOL_STATUS,
    TOOL_PROBE_TARGET,
    TOOL_READ_PINS,
    TOOL_SYSTEM_FIRMWARE_STATUS,
    TOOL_SERVO_LIST,
    TOOL_SERVO_ATTACH,
    TOOL_SERVO_WRITE_ANGLE,
    TOOL_SERVO_WRITE_US,
    TOOL_SERVO_DETACH,
    TOOL_LOGIC_ANALYZER_ARM,
    TOOL_LOGIC_ANALYZER_START,
    TOOL_LOGIC_ANALYZER_STATUS,
    TOOL_LOGIC_ANALYZER_DATA,
    TOOL_PROGRAMMER_READ,
    TOOL_PROGRAMMER_FLASH,
    TOOL_PROGRAMMER_RESET,
    TOOL_PROGRAMMER_UNBRICK,
    TOOL_CATALOG_LOOKUP,
    TOOL_I2C_SCAN,
    TOOL_CAN_STATUS,
    TOOL_CAN_SEND,
    TOOL_CAN_SNIFF,
    TOOL_CAN_SET_TERMINATION,
    TOOL_SPI_TRANSFER,
    TOOL_UART_COMMAND,
    TOOL_PINS_STATUS,
    TOOL_PINS_CLAIM,
    TOOL_PINS_RELEASE,
    TOOL_NEOPIXEL_FILL,
    TOOL_NEOPIXEL_SET,
    TOOL_NEOPIXEL_CLEAR,
    TOOL_NEOPIXEL_PATTERN,
    TOOL_NEOPIXEL_ANALYZE,
    TOOL_NEOPIXEL_TRANSMIT,
    TOOL_TARGET_DIAGNOSE,
    TOOL_INSPECT_FIRMWARE,
    TOOL_SNIFF_BUS,
    TOOL_APP_LIST,
    TOOL_APP_INSTALL,
    TOOL_POWER_CONTROL,
    TOOL_POWER_CYCLE,
    TOOL_EMERGENCY_SAFE_STATE,
    TOOL_PROFILE_LIST,
    TOOL_PROFILE_APPLY,
    TOOL_SWO_READ,
    TOOL_SENSOR_SCAN,
    TOOL_SENSOR_READ_REG,
    TOOL_SENSOR_WRITE_REG,
    TOOL_SENSOR_POLL,
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
