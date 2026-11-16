# dormant-to-swd Specification

## Purpose
TBD - created by archiving change rewrite-swd-pio-engine. Update Purpose after archive.
## Requirements
### Requirement: Dormant-to-SWD wake sequence

The SWD engine SHALL implement the ARM ADIv5 and ADIv6 dormant-to-SWD wake sequence to support targets that may be in dormant state, invoking it dynamically during target attachment and DPIDR retry loops.

#### Scenario: Target is in dormant state
- **WHEN** SWD initialization or retry is called
- **AND** the target may be in dormant or JTAG state
- **THEN** the engine shall send:
  1. Line reset: 56+ HIGH clock cycles on SWCLK
  2. JTAG-to-SWD switch: 0xE79E (16 bits, LSB first)
  3. Line reset again: 56+ HIGH clock cycles
  4. Dormant-to-SWD: 128-bit Selection Alert sequence
  5. SWD protocol activation code: 0x1A
  6. Final line reset: 56+ HIGH clock cycles

#### Scenario: Target responds to dormant wake
- **WHEN** the dormant-to-SWD sequence is sent
- **AND** the target is functional
- **THEN** a subsequent DPIDR read shall return a valid 32-bit IDCODE (such as 0x0BE12477 for Cortex-M33)

#### Scenario: Target is not in dormant state
- **WHEN** the dormant-to-SWD sequence is sent
- **AND** the target is already in SWD mode
- **THEN** the sequence shall be ignored harmlessly
- **AND** subsequent SWD operations shall succeed

