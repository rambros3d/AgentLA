## ADDED Requirements

### Requirement: One active capture bank at a time
The system SHALL allow exactly one capture bank claim to be active at any time.

#### Scenario: Second capture claim
- **WHEN** a capture bank is already active and another capture bank is requested
- **THEN** the second claim SHALL fail and the active bank SHALL remain unchanged

### Requirement: Span-based capture claim
The system SHALL allow a capture claim to declare a subset of capture pins, while reserving the inclusive physical span from the lowest to highest selected pin as exclusive capture ownership.

#### Scenario: Subset request
- **WHEN** a capture claim requests GPIO12 and GPIO16 as capture channels
- **THEN** the claim SHALL succeed only if GPIO12 through GPIO16 are all available, and metadata SHALL identify the requested pins plus the 13-bit span width

#### Scenario: Nested claim conflict
- **WHEN** another flexible role attempts to claim any pin inside an active capture span
- **THEN** that claim SHALL fail because the span is exclusively owned

### Requirement: Capture channel metadata is discoverable
The system SHALL associate each capture claim with the selected physical pins, selected-bit mask, original span width, and trigger index.

#### Scenario: Capture metadata retrieval
- **WHEN** a host reads the active capture bank metadata
- **THEN** the system SHALL return the selected pins, span width, and trigger pin used for that claim

### Requirement: Explicit trigger ownership
The system SHALL require a capture claim to explicitly declare the trigger.
The trigger SHALL either be one of the claimed pins or be explicitly declared as unused.

#### Scenario: Claim without trigger state
- **WHEN** a capture claim does not specify a trigger mode
- **THEN** the claim SHALL fail before hardware initialization

#### Scenario: Trigger declared unused
- **WHEN** a caller intentionally claims a bank without trigger detection
- **THEN** the claim SHALL succeed and trigger-related logic SHALL remain inactive

### Requirement: Safe subset interpretation
The system SHALL not present non-selected pins inside a span as independent channels while retaining their span occupancy for contention rules.

#### Scenario: Neighbor within an active span
- **WHEN** GPIO13 is inside an active capture span but is not declared as a selected channel
- **THEN** GPIO13 SHALL remain unavailable to other claims for the duration of the span claim

### Requirement: Initial capture width bound
The system SHALL initially support capture spans no wider than 8 pins unless a later measured configuration demonstrates additional DMA and PIO headroom.

#### Scenario: Request larger than supported width
- **WHEN** a capture claim requests a span wider than the initial supported limit
- **THEN** the claim SHALL fail with a capacity reason before PIO or DMA initialization

### Requirement: ADC pins as dependent analog capabilities
The system SHALL validate ADC or scope claims against analog-capable pins within the flexible pool only.

#### Scenario: Analog-only flex claim
- **WHEN** a scoped role claims a digital capability on GPIO10
- **THEN** the base digital claim SHALL be considered, but the analog capability SHALL be rejected because GPIO10 is not analog-capable

#### Scenario: Analog plus digital
- **WHEN** an analog role successfully claims GPIO27 as both digital capture and ADC input
- **THEN** the scope role SHALL gain both capabilities under the same ownership lifecycle and be released together