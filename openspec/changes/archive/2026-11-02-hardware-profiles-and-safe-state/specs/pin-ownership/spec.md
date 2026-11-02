## MODIFIED Requirements

### Requirement: Blank start
The system SHALL initialize the flexible ownership registry without any Adapter, Capture, Scope, or ADC claims, ensuring all flexible and zone GPIO pins remain in safe High-Z input mode with pulls disabled until claimed.

#### Scenario: Fresh boot state
- **WHEN** firmware starts in any mode that does not include Programmer-only initialization
- **THEN** no flexible role SHALL be marked as owned or active, and all physical pins SHALL remain tri-stated inputs until explicitly claimed by a profile or instrument

## ADDED Requirements

### Requirement: Safe Atomic Handover Across Profile Transitions
The pin ownership registry SHALL provide an atomic `pin_owner_quiesce_and_reset()` function that safely revokes all existing claims, releases drivers, tri-states all physical pins to SIO inputs, and waits for bus quiescence before granting claims for a newly loaded profile.

#### Scenario: Switching active profiles
- **WHEN** a new profile is applied over an active configuration
- **THEN** all prior driver and observer claims SHALL be revoked, physical pins driven to High-Z inputs, and new claims established atomically
