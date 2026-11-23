# Spec Delta

## MODIFIED Requirements

### Requirement: Chip Catalog and Memory Map Lookup
The `agentla-programmer` skill SHALL guide the agent to query the chip catalog before issuing flash commands to ensure correct memory addresses and sector boundaries.

#### Scenario: Lookup flash boundaries before programming
- **WHEN** the agent prepares to flash a target chip (e.g. STM32F401)
- **THEN** it executes `catalog_lookup` to obtain Flash base (`0x08000000`), sector layout, and RAM buffers.

### Requirement: Universal Flash Programming and Verification
The `agentla-programmer` skill SHALL guide the agent to perform firmware flash programming directly over host USB interfaces (CMSIS-DAP or ST-Link via probe-rs / pyocd) rather than HTTP REST or MCP programmer tools, followed by verification.

#### Scenario: Flash payload via direct USB probe
- **WHEN** writing firmware images to target memory
- **THEN** the skill directs the agent to invoke direct USB flashing tools (such as `probe-rs run` or `pyocd flash`), followed by verification.

### Requirement: Target Unbricking and Unlock Sequences
The `agentla-programmer` skill SHALL specify exact unbricking procedures for locked devices via direct USB debug probe access (e.g., STM32 RDP Level 1, nRF APPROTECT, RP2350 OTP lock).

#### Scenario: Rescue locked target silicon via USB debug probe
- **WHEN** SWD connection fails due to active security protection or misconfigured GPIOs
- **THEN** the skill guides the agent to execute direct USB unlock sequences via host debug tools (e.g., `probe-rs reset` with connect-under-reset or CTRL-AP mass erase).
