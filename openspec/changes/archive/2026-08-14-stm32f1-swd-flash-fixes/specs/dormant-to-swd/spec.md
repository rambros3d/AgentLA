## MODIFIED Requirements

### Requirement: SWD connect sequence for SWJ-DP targets
The SWD engine SHALL bring up the debug interface deterministically for JTAG-default SWJ-DP targets (e.g., STM32F1, which powers up in JTAG mode and does not support the ADIv5.2 dormant state), as well as for ADIv5.2 dormant-capable targets.

#### Scenario: Connect to a JTAG-default SWJ-DP target
- **WHEN** `swd_read_idcode()` is called with a target that powers up in JTAG mode (STM32F1 SWJ-DP)
- **THEN** the system SHALL perform the JTAG-to-SWD switch — line reset, 16-bit `0xE79E` select sequence, line reset — before relying on the target responding
- **AND** the system SHALL read DPIDR after the switch and succeed without requiring a dormant-state wake

#### Scenario: Connect to a dormant-capable target
- **WHEN** `swd_read_idcode()` is called with a target that supports the dormant state (ADIv5.2+, e.g., CH32V203 RVSWD)
- **THEN** the system SHALL still be able to wake it via the dormant-to-SWD sequence (line reset, selection alert, activation code, line reset)
- **AND** SHALL NOT regress connection for these targets

#### Scenario: Connect ordering
- **WHEN** the target does not respond to the initial DPIDR read
- **THEN** the system SHALL try the alternate switch sequence (JTAG-to-SWD if dormant was tried first, or vice versa) and re-read DPIDR before giving up
- **AND** a line reset SHALL precede any switch sequence to return the interface to a known state

#### Scenario: JTAG-to-dormant transition (unified path, if adopted)
- **WHEN** the dormant-to-SWD sequence is used on a target that may be in JTAG mode
- **THEN** the system SHALL first transition the JTAG TAP to the dormant state using the 44-bit JTAG-to-dormant sequence (`0x1F` 5 bits, `0x33BBBBBA` 31 bits, `0xFF` 8 bits) before sending the 128-bit selection alert
