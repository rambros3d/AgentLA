## ADDED Requirements

### Requirement: RP2350 Embassy Runtime Initialization
The firmware SHALL initialize the RP2350 hardware peripherals, system clocks (150 MHz), and async Embassy executor on Cortex-M33.

#### Scenario: Successful board boot
- **WHEN** the RP2350 probe is powered on or reset
- **THEN** system clocks are locked, Embassy runtime initializes, and background tasks are spawned on the primary executor

### Requirement: Watchdog Supervisor
The firmware SHALL configure an internal hardware watchdog to reset the board if the async executor is starved or deadlocked.

#### Scenario: Regular watchdog petting
- **WHEN** async tasks are executing normally within the timeout window
- **THEN** the supervisor task pets the watchdog periodically without triggering a system reset

#### Scenario: Hang recovery
- **WHEN** any critical hardware loop blocks or panics
- **THEN** the watchdog timer expires and cleanly reboots the RP2350
