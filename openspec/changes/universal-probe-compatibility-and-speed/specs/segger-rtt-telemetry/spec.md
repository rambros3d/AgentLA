## ADDED Requirements

### Requirement: Segger RTT signature discovery
The system SHALL scan target SRAM to detect and parse the SEGGER RTT control block without requiring user-provided memory addresses.

#### Scenario: Successful RTT discovery
- **WHEN** an RTT scan is initiated on a running or halted target
- **THEN** the system searches the SRAM window for the 16-byte `"SEGGER RTT\0\0\0\0\0\0"` signature, parses the control block pointers, and returns `OK RTT FOUND` with buffer descriptor metadata

#### Scenario: RTT signature absent
- **WHEN** target SRAM does not contain the RTT control block
- **THEN** the system returns `ERR RTT NOT FOUND` without halting or disrupting target execution

### Requirement: Non-intrusive background buffer draining
The system SHALL read log characters from target RTT up-buffers using non-intrusive CoreSight memory access while the target CPU continues running.

#### Scenario: Continuous telemetry stream
- **WHEN** target firmware logs characters into RTT up-buffer 0
- **THEN** the system reads newly appended bytes via SWD, advances `aUp[0].RdOff`, and delivers the log stream to the WebUI terminal and WebSocket clients
