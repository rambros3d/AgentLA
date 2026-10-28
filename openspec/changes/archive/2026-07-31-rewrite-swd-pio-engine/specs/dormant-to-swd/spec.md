## ADDED Requirements

### Requirement: Dormant-to-SWD wake sequence

The SWD engine shall implement the ARM ADIv5 dormant-to-SWD wake sequence to support targets that may be in dormant state.

#### Scenario: Target is in dormant state
- **WHEN** SWD initialization is called
- **AND** the target may be in dormant or JTAG state
- **THEN** the engine shall send:
  1. Line reset: 56+ HIGH clock cycles on SWCLK
  2. JTAG-to-SWD switch: 0xE79E (16 bits, LSB first)
  3. Line reset again: 56+ HIGH clock cycles
  4. Dormant-to-SWD: 128-bit Selection Alert sequence
  5. Activation code

#### Scenario: Target responds to dormant wake
- **WHEN** the dormant-to-SWD sequence is sent
- **AND** the target is functional
- **THEN** a subsequent DPIDR read shall return a valid 32-bit IDCODE

#### Scenario: Target is not in dormant state
- **WHEN** the dormant-to-SWD sequence is sent
- **AND** the target is already in SWD mode
- **THEN** the sequence shall be ignored harmlessly
- **AND** subsequent SWD operations shall succeed
