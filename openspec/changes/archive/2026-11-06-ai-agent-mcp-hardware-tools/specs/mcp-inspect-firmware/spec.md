## ADDED Requirements

### Requirement: Vector table and layout inspection
The system SHALL provide an autonomous MCP tool `mcp_inspect_firmware` that validates initial Stack Pointer boundaries, Thumb mode invariant, and option bytes from target flash.

#### Scenario: Valid vector table
- **WHEN** `mcp_inspect_firmware` is invoked on correctly linked firmware
- **THEN** the system returns initial SP, Reset Vector, verifies SP is within physical SRAM bounds, verifies Thumb mode bit is set (LSB=1), and reports `valid: true`

#### Scenario: Invalid stack pointer detection
- **WHEN** the firmware Initial SP points outside the physical SRAM bounds defined in the loaded LittleFS algorithm header
- **THEN** the system flags `sp_valid: false` with warning describing the memory map mismatch

#### Scenario: Missing Thumb bit detection
- **WHEN** the Reset Vector has LSB=0 (even address)
- **THEN** the system flags `reset_vector_thumb_valid: false` and warns that ARM core will trigger HardFault on reset
