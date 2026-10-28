## ADDED Requirements

### Requirement: Fixed programmer pin block
The system SHALL reserve GPIO0 through GPIO9 as fixed programmatic resources and SHALL NOT allow any flexible role to claim them.

#### Scenario: Attempted claim on programmer pin
- **WHEN** a caller attempts to claim GPIO0 through GPIO9 for an Adapter, Capture, Scope, or ADC role
- **THEN** the claim SHALL fail and the registry SHALL leave the requested pins unassigned

### Requirement: Flexible pin pool is explicitly bounded
The system SHALL allow flexible roles to claim only GPIO10 through GPIO29.

#### Scenario: Valid flexible pin claim
- **WHEN** a caller attempts to claim a pin in GPIO10 through GPIO29 with a matching capability
- **THEN** the claim SHALL be considered for ownership if all other constraints pass

#### Scenario: Invalid flexible pin claim
- **WHEN** a caller attempts to claim a pin outside GPIO10 through GPIO29 for a flexible role
- **THEN** the claim SHALL fail because that pin is outside the flexible pool

### Requirement: Blank start
The system SHALL initialize the flexible ownership registry without any Adapter, Capture, Scope, or ADC claims.

#### Scenario: Fresh boot state
- **WHEN** firmware starts in any mode that does not include Programmer-only initialization
- **THEN** no flexible role SHALL be marked as owned or active until explicitly claimed

### Requirement: Explicit claim handles
The system SHALL provide an explicit ownership handle for each successful runtime claim, and all flexible-role operations SHALL require an active handle to that exact resource.

#### Scenario: Use without claim
- **WHEN** an Adapter, Capture, Scope, or ADC operation is requested without a handle for its required resources
- **THEN** the operation SHALL fail before performing hardware setup or modifying pins

### Requirement: Atomic claim transactions
The system SHALL acquire every resource required by a claim in a single transaction and SHALL make no partial resource assignment when any dependency is unavailable.

#### Scenario: One dependency unavailable
- **WHEN** a claim requires multiple pins or instances and one of them is unavailable
- **THEN** the system SHALL release any resources held for that transaction and return a failure

### Requirement: RP2350 capability validation
The system SHALL validate role, pin, and function compatibility using the documented RP2350 function support and valid package constraints.

#### Scenario: Incompatible role and pin
- **WHEN** a caller attempts to use a pin for a function that the documented RP2350 table does not permit
- **THEN** the claim SHALL fail and the pin SHALL retain its prior ownership state

### Requirement: Active driver ownership
The system SHALL permit no more than one active output driver for a physical pin at a time, while allowing read-only observation roles as specified.

#### Scenario: Conflicting driver claim
- **WHEN** two independent requests attempt to claim the same GPIO as a driver
- **THEN** the second request SHALL fail with conflict information and leave the first ownership intact

### Requirement: Explicit release with deterministic outcome
The system SHALL support explicit `RELEASE` of an owned resource and SHALL report whether release succeeded, was delayed by quiesce, or failed. Repeated release requests for the same handle SHALL be idempotent once quiesce is in progress or complete.

#### Scenario: Release with active role
- **WHEN** a caller releases an active role
- **THEN** the system SHALL attempt the required quiesce, then release the resource and report the resulting status

#### Scenario: Repeated release
- **WHEN** a caller releases the same resource while a prior release is still in progress or already completed
- **THEN** the system SHALL return the current release status without initiating another quiesce sequence

#### Scenario: Release after quiesce failure
- **WHEN** release cannot safely complete its quiesce sequence
- **THEN** the system SHALL leave ownership unchanged and report a failure

### Requirement: All-or-nothing bundle claims
The system SHALL expand transparent bundles into explicit claims and SHALL either acquire every dependency or fail without retaining a partial claim.

#### Scenario: One bundle dependency unavailable
- **WHEN** a bundle request contains at least one unavailable resource
- **THEN** none of the bundle's resources SHALL remain reserved after the request returns

### Requirement: Shared read-only input observation

The system SHALL allow multiple read-only observation roles to observe a flexible pin that is already assigned, provided none of them drives the pin as an output and no electrical side effect is introduced.

#### Scenario: Safe observation of an input pin
- **WHEN** a second role requests read-only access to a flexible pin already in use as an input
- **THEN** the claim SHALL succeed as an observation while the original role's driver authorization is preserved

#### Scenario: Attempted output while one already owns the pin
- **WHEN** a new role requests an output or drive-capable role for a pin already carrying an active driver
- **THEN** the claim SHALL fail and the pin SHALL retain its existing ownership state

### Requirement: Runtime pin inventory
The system SHALL expose the flexible pins and analog-capable pins it can safely offer as runtime inventory.

#### Scenario: Package reports analog pins
- **WHEN** firmware starts on a supported RP2350 board
- **THEN** the inventory SHALL mark GPIO26 through GPIO29 as analog-capable and other flexible pins accordingly

### Requirement: Interrupt ownership resource
The system SHALL represent interrupt and edge-callback registration as an independent ownership resource tied to an otherwise valid pin or capture claim.

#### Scenario: Interrupt registration succeeds
- **WHEN** an interrupt request is made for a valid owned pin or capture claim
- **THEN** the system SHALL create a separate ownership resource with its own lifecycle

#### Scenario: Interrupt released
- **WHEN** interrupt ownership is released
- **THEN** callback registration SHALL be removed and the capability SHALL become unavailable

### Requirement: Safe read-only discovery

The system SHALL support an explicit discovery mode that only observes supported bus input ranges and SHALL NOT mutate any claimed pins or issue distracting bus transactions.

#### Scenario: Discovery requests claim
- **WHEN** discovery starts while a pin is already claimed
- **THEN** it SHALL refrain from changing its function or interrupting its active role

### Requirement: No backward compatibility in flexible-role commands

The system SHALL treat new flexible-role explicit claim handles as the only valid API and SHALL NOT infer historical static pin assignments.

#### Scenario: Legacy assumption
- **WHEN** a command depends only on historical fixed adapter or capture pins
- **THEN** the command SHALL fail to operate until a valid claim handle is used