## MODIFIED Requirements

### Requirement: Fixed programmer pin block
The firmware SHALL reserve GPIO 4 through GPIO 14 as fixed programmatic resources owned by the three-port programmer bridge (Port 1 SWD: 4/5/6, Port 2 RVSWD: 8/9/10, Port 3 SWIO: 12/13/14) and SHALL NOT allow any flexible role to claim them.

#### Scenario: Attempted claim on programmer pin
- **WHEN** a caller attempts to claim GPIO 4 through GPIO 14 for an Adapter, Capture, Servo, or user-GPIO role
- **THEN** the claim SHALL fail with a conflict error and the registry SHALL leave the requested pins assigned to the programmer bridge

### Requirement: Flexible pin pool is explicitly bounded
The firmware SHALL allow flexible roles to claim only GPIO 0 through 3, GPIO 11, and GPIO 15 through GPIO 29 (GPIO 26–29 additionally analog-capable). GPIO 25 (status LED) and GPIO 30/31 (USB pads) SHALL be claimed by no flexible role.

#### Scenario: Valid flexible pin claim
- **WHEN** a caller attempts to claim a pin in the flexible pool with a matching capability
- **THEN** the claim SHALL be considered for ownership if all other constraints pass

#### Scenario: Invalid flexible pin claim
- **WHEN** a caller attempts to claim GPIO 25, GPIO 30, GPIO 31, or a programmer-bridge pin for a flexible role
- **THEN** the claim SHALL fail with an explicit conflict or out-of-pool error

### Requirement: Blank start
The firmware SHALL initialize the flexible ownership registry with no claims at boot. At boot, before the registry grants any claim, all flexible pins SHALL be placed in safe high-Z state: function SIO, direction input, pulls disabled.

#### Scenario: Fresh boot state
- **WHEN** the firmware starts, either before a boot-default profile is recorded or without one
- **THEN** no flexible role SHALL be marked as owned, and all flexible pins SHALL be tri-stated inputs

### Requirement: Explicit release with deterministic outcome
The firmware SHALL support explicit release of an owned claim by handle. Release SHALL synchronously quiesce every pin freed by the claim (function SIO, direction input, pulls disabled) before reporting success, and repeated release of the same handle SHALL be idempotent (returns the current status without further hardware writes).

#### Scenario: Release with active role
- **WHEN** a caller releases an active role by handle
- **THEN** the freed pins SHALL be quiesced to SIO/in/no-pull and the caller SHALL receive a success status in the same synchronous call

#### Scenario: Repeated release
- **WHEN** a caller releases the same handle again
- **THEN** the system SHALL return the current status without performing additional hardware writes

## REMOVED Requirements

### Requirement: Shared read-only input observation
**Reason**: The C implementation's observer path was unreachable in practice — its claimability predicate only ever admitted GPIO_FUNC_SIO and no firmware component ever created an observer claim, so no consumer or test ever exercised it.
**Migration**: Output-free "input listener" claims can be reintroduced as a narrow extension when a real consumer exists (e.g., logic-analyzer trigger inputs); until then any second claim on an owned pin is a conflict.

### Requirement: Interrupt ownership resource
**Reason**: No C caller ever used the IRQ attach/detach lifecycle; the Rust port does not manage vector registry state through the pin registry.
**Migration**: Future consumers needing per-pin callbacks will reintroduce this as an added requirement with a real use case.
