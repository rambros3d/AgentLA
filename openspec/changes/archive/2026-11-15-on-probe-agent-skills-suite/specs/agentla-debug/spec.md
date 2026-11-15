## ADDED Requirements

### Requirement: Hardware Triage and Electrical Health Verification
The `agentla-debug` skill SHALL mandate electrical rail sensing before asserting digital debug signals to protect target silicon.

#### Scenario: Verify target rail before SWD line drive
- **WHEN** the agent runs the debug triage playbook
- **THEN** it reads target supply voltage telemetry and halts with a warning if VTGT is below 1.65V or above 3.6V.

### Requirement: CoreSight AP and Target Identification
The `agentla-debug` skill SHALL guide the agent through CoreSight debug port discovery, DPIDR reading, AP scanning, and CPUID register verification.

#### Scenario: Autonomous target detection
- **WHEN** the agent initiates debugging on an unknown or unresponsive Cortex-M target
- **THEN** it invokes `mcp_target_diagnose` to retrieve DPIDR, CPUID, and AP table, identifying the silicon core.

### Requirement: HardFault Root-Cause Analysis
The `agentla-debug` skill SHALL provide step-by-step diagnostic parsing of Cortex-M CFSR, HFSR, MMFAR, BFAR, and vector table stack pointer alignment.

#### Scenario: Decode bus fault / memory management fault
- **WHEN** the target processor halts in a HardFault handler
- **THEN** the skill directs the agent to read SCB fault registers, decode whether a PRECISERR, BFARVALID, or STKERR occurred, inspect the stacked return PC, and pinpoint the offending instruction.
