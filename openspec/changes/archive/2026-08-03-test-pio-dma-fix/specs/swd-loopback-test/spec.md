## ADDED Requirements

### Requirement: PIO wire-level loopback test
The system SHALL provide a CLI command (`LOOPBACK`) that verifies PIO can drive and read GPIO pins by toggling SWCLK and reading SWDIO with an external jumper connecting the two pins.

#### Scenario: Loopback with jumper installed
- **WHEN** the user sends `LOOPBACK\r\n` on CDC0 with a jumper wire connecting GPIO4 (SWCLK) to GPIO5 (SWDIO)
- **THEN** the system configures SWCLK as PIO output, sends 8 toggle patterns, configures SWDIO as PIO input, reads back the patterns, and reports PASS if all bits match

#### Scenario: Loopback without jumper
- **WHEN** the user sends `LOOPBACK\r\n` on CDC0 without a jumper
- **THEN** the system reports FAIL with "no signal detected on SWDIO"

#### Scenario: Loopback PIO SM isolation
- **WHEN** the loopback test runs
- **THEN** it uses the same PIO1 SM0 and DMA channel as the SWD engine, verifying the full DMA→PIO→GPIO→GPIO→PIO→RX path without involving any SWD protocol logic

### Requirement: SWD basic connectivity test
The system SHALL provide a CLI command (`SWDTEST`) that performs the minimal SWD initialization sequence: dormant-to-SWD wake, line reset, and DPIDR read, reporting the result.

#### Scenario: SWDTEST with target connected
- **WHEN** the user sends `SWDTEST\r\n` on CDC0 with a CH32V203 connected via RV-SWD
- **THEN** the system performs dormant-to-SWD, line reset, reads DPIDR, and prints the 32-bit IDCODE value (non-zero, non-0xFFFFFFFF for a valid target)

#### Scenario: SWDTEST with no target
- **WHEN** the user sends `SWDTEST\r\n` on CDC0 with no target connected
- **THEN** the system completes the SWD sequence and reports DPIDR = 0x00000000 or 0xFFFFFFFF (no target detected)

#### Scenario: SWDTEST PIO DMA does not hang
- **WHEN** the SWDTEST command runs
- **THEN** all PIO DMA transfers complete within their timeouts and the main loop is never blocked for more than 100ms per operation

### Requirement: SWDTEST output on CDC1 debug channel
The system SHALL write detailed SWD test progress (dormant-to-SWD, line reset, DPIDR read status) to CDC1 (`/dev/ttyACM1`) using `dbg_printf`, while the final PASS/FAIL result goes to CDC0.

#### Scenario: CDC1 shows SWD test progress
- **WHEN** SWDTEST is running
- **THEN** CDC1 receives lines like `[SWD] dormant-to-swd start`, `[SWD] line_reset done`, `[SWD] DPIDR read: ack=0 idcode=0x1BC03405`
