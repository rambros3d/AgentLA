## ADDED Requirements

### Requirement: Autonomous Developer Playbook Definition
The system SHALL provide an autonomous firmware developer skill specification (`agentla-dev`) that guides an agent through target-probe wiring, code generation, programming, and physical signal verification.

#### Scenario: Autonomous closed-loop workflow execution
- **WHEN** an AI agent reads the `agentla-dev` skill
- **THEN** it receives a structured sequence: (1) query target catalog, (2) output wiring instructions to the user, (3) verify rail voltage and SWD connectivity, (4) compile and flash firmware, (5) acquire logic analyzer pins via Pin Arbiter, (6) verify physical pin toggling/frequency, (7) report success or self-heal on failure.

### Requirement: Target Wiring Guidance Protocol
The `agentla-dev` skill SHALL instruct the AI agent to explicitly verify and guide target pin connections before executing flashing or bus transactions.

#### Scenario: User pinout and wiring prompt
- **WHEN** the agent initiates a new firmware project for a target MCU
- **THEN** the skill directs the agent to instruct the user on connecting SWCLK, SWDIO, GND, VTGT (3.3V sense), and the verification test pins (e.g., LED/GPIO to AgentLA D0-D7).

### Requirement: Closed-Loop Physical Verification
The `agentla-dev` skill SHALL require the agent to verify physical outputs using the logic analyzer or protocol bridges rather than assuming software success.

#### Scenario: LED blink frequency validation
- **WHEN** the agent develops a 1Hz blink firmware
- **THEN** the agent claims the corresponding probe channel via Pin Arbiter, triggers a logic analyzer capture, computes high/low pulse duration, and validates that the measured period is 1000ms ± 5%.
