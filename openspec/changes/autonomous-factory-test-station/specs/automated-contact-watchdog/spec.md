## ADDED Requirements

### Requirement: Autonomous Pogo Pin Contact Sensing
The firmware SHALL continuously sample the target voltage sense input (VTARGET on GP47 / ADC7) and trigger a new test cycle when the voltage exceeds a configurable contact threshold for a stable debounce period.

#### Scenario: PCB clamped into bed of nails
- **WHEN** an unpowered or powered target PCB is pressed onto the pogo pins and VTARGET exceeds 1.2V for 150ms
- **THEN** the firmware SHALL transition from IDLE state to PRE_FLIGHT_CHECK and initiate the active test sequence

### Requirement: Pre-Flight Sub-Millisecond Short Circuit Protection
The firmware SHALL measure quiescent target current via the INA180 current sense circuit (GP46 / ADC6) immediately upon contact, and trip target power if the current exceeds the pre-flight safety threshold.

#### Scenario: Target board with dead short placed on jig
- **WHEN** the target power rail is energized and current draw exceeds the recipe safety limit (e.g. > 100mA) within the first 10ms
- **THEN** the firmware SHALL shut off the target power MOSFET within 1 millisecond, abort the cycle, and report `ERR_SHORT_CIRCUIT`

### Requirement: Safe De-Energization on Premature Unclamp
The firmware SHALL detect premature board removal during an active test cycle, instantly discharge residual voltage, and enter a safe High-Z state.

#### Scenario: Operator unclamps fixture during active flashing
- **WHEN** VTARGET drops below 0.8V during an active programming or test operation
- **THEN** the firmware SHALL immediately abort the flash loop, set all programmer and bus pins to High-Z, and record a failed test status in the database
