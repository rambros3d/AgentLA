## ADDED Requirements

### Requirement: Chip Catalog and Memory Map Lookup
The `agentla-programmer` skill SHALL guide the agent to query the chip catalog before issuing flash commands to ensure correct memory addresses and sector boundaries.

#### Scenario: Lookup flash boundaries before programming
- **WHEN** the agent prepares to flash a target chip (e.g. STM32F401)
- **THEN** it executes `catalog_lookup` to obtain Flash base (`0x08000000`), sector layout, and RAM buffers.

### Requirement: Universal Flash Programming and Verification
The `agentla-programmer` skill SHALL define chunked flash programming sequences with CRC/read-back verification.

#### Scenario: Flash payload with verification
- **WHEN** writing firmware images to target memory
- **THEN** the skill directs the agent to invoke `programmer_flash` in chunks, followed by `programmer_read` or CRC verification to guarantee integrity.

### Requirement: Target Unbricking and Unlock Sequences
The `agentla-programmer` skill SHALL specify exact unbricking procedures for locked devices (e.g. STM32 RDP Level 1, nRF APPROTECT, RP2350 OTP lock).

#### Scenario: Rescue locked target silicon
- **WHEN** SWD connection fails due to active security protection or misconfigured GPIOs
- **THEN** the skill guides the agent to execute `programmer_unbrick` with family-specific unlock sequences (e.g., CTRL-AP mass erase or power-glitch reset).
