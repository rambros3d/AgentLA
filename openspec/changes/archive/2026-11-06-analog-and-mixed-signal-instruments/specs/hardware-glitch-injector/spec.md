# hardware-glitch-injector Specification

## ADDED Requirements

### Requirement: Cycle-Accurate Glitch Pulse Generation
The system SHALL provide a PIO-driven fault injection engine capable of outputting voltage or clock glitches with nanosecond-resolution pulse width down to 13.3 ns.

#### Scenario: Firing an armed glitch
- **WHEN** user arms a glitch on GP20 with trigger pin GP14 (rising edge), delay 1250 ns, and pulse width 33 ns
- **THEN** system detects rising edge on GP14, waits 1250 ns, drives GP20 low for 33 ns, and disarms automatically

### Requirement: Glitch Status and Safety
The system SHALL keep glitch pins tri-stated or in safe idle state when disarmed and provide arming status via REST API.

#### Scenario: Checking glitch status
- **WHEN** user queries `GET /api/v1/glitch/status`
- **THEN** system returns armed state, trigger count, and active timing parameters
