## Why

AI agents (such as Claude, DeepMind, or automated CI systems) interacting with embedded hardware currently face high friction: diagnosing a non-booting board requires orchestrating dozens of low-level queries (reading pin banks, querying status registers, guessing memory maps, parsing raw hex words) to piece together root causes. 

This change introduces three high-level, composite **AI Agent Hardware Tools** to AgentLA's on-chip Model Context Protocol (MCP) server:
1. `mcp_target_diagnose`: A one-shot diagnostic tool that inspects physical rail voltage, tests pin continuity/pull-up state, probes CoreSight DPIDR/CPUID, and performs automatic HardFault root-cause analysis.
2. `mcp_inspect_firmware`: A vector table and layout inspector that validates initial Stack Pointer boundaries against target SRAM (using LittleFS CMSIS-Pack metadata), verifies Thumb mode bit compliance (LSB=1), and reads option bytes.
3. `mcp_sniff_bus`: An autonomous bus transaction sniffer that monitors I2C/SPI lines for a defined window, decodes active packets via in-firmware decoders, and returns a structured JSON transaction log.

## What Changes

- **Universal Target Diagnostic Engine (`mcp_target_diagnose`)**:
  - Measures target supply rail voltage $V_{\text{target}}$ via ADC (detecting nominal $3.3\text{V}$, brownout, or unpowered states).
  - Toggles internal pad pull-downs on `SWCLK`, `SWDIO`, and `nRST` to distinguish hard-driven, pulled-up, floating, or shorted pins.
  - Attaches over SWD/RVSWD, probes DPIDR, CPUID, and CoreSight identifiers.
  - Analyzes CPU execution state: reads `DHCSR`, `CFSR`, `HFSR`, `BFAR`, `PC`, and `LR` to explain exact HardFault root causes (e.g. unclocked peripheral access, unaligned load, divide by zero).
- **Universal Firmware Inspector (`mcp_inspect_firmware`)**:
  - Reads flash vector table entries (SP, Reset Vector, NMI, HardFault).
  * Validates Stack Pointer bounds against target SRAM boundaries resolved dynamically from LittleFS `AlgoBlobHeader` (`/system/algo/<target>.bin`).
  * Enforces the ARM Thumb mode invariant (`Reset_Vector & 1 == 1`) to catch immediate vector crash bugs.
  * Reads target option bytes (RDP level, hardware watchdog flags).
- **Autonomous Bus Sniffer (`mcp_sniff_bus`)**:
  - Arms logic analyzer DMA capture for requested duration (50 ms – 1000 ms) on I2C (SDA/SCL) or SPI (MOSI/MISO/SCK/CS).
  - Feeds captured buffers into in-firmware protocol decoders and aggregates active device addresses, read/write frames, and NACK events into structured JSON.
- **MCP Server Surface Updates**:
  - Registers the new composite tools in `tools/list` schema and dispatches them in `tools/call` over `http://10.10.10.10/mcp`.

## Capabilities

### New Capabilities
- `mcp-target-diagnose`: One-shot autonomous diagnostic tool inspecting target electrical power, pin drive integrity, CoreSight identification, and CPU HardFault register analysis.
- `mcp-inspect-firmware`: Target vector table inspection, stack pointer boundary validation against LittleFS algorithm headers, and Thumb mode verification.
- `mcp-sniff-bus`: Autonomous bus sniffer monitoring I2C and SPI lines, returning decoded transactions and address inventories.

### Modified Capabilities
- `mcp-protocol-surface`: Extends the MCP JSON-RPC tool catalog and dispatch table with the three composite hardware tools.

## Impact

- **Firmware (`firmware-rs`)**:
  - `src/mcp.rs`: Registers new tools in `tools/list` and dispatches to diagnostic modules.
  * `src/programmer/target/diagnostic.rs`: New module implementing universal electrical, CoreSight fault, and vector table inspectors.
  * `src/analyzer.rs` & `src/decoder.rs`: Provides capture-and-decode window helper for bus sniffing.
- **MCP Consumers**:
  - AI agents, CLI scripts, and web desktop tools can invoke high-level hardware diagnostics via single JSON-RPC calls.
