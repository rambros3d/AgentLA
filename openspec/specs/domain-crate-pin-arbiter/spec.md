# domain-crate-pin-arbiter Specification

## Purpose
TBD - created by archiving change modularize-pin-arbiter-crate. Update Purpose after archive.
## Requirements
### Requirement: Standalone Pin Arbiter Crate Structure
The system SHALL provide a dedicated crate `crates/agentla-pin-arbiter` configured as a `#![no_std]` library with optional `defmt` and `std` features.

#### Scenario: Compiling and testing on host targets
- **WHEN** building `agentla-pin-arbiter` on an x86_64 host with `cargo test --package agentla-pin-arbiter`
- **THEN** all arbitration and conflict detection unit tests compile and pass.

### Requirement: All-or-Nothing Claim Transactions
The `agentla-pin-arbiter` crate SHALL enforce that multi-pin claims either succeed completely or leave all pins unchanged if any pin is already owned or lacks the required capability.

#### Scenario: Contested multi-pin claim
- **WHEN** a driver attempts to claim pins `[12, 13, 14]` but pin `14` is already claimed
- **THEN** the claim returns `Err(ClaimError::Conflict)` and pins `12` and `13` remain unassigned.

### Requirement: Pin Quiesce Notification on Release
The `agentla-pin-arbiter` crate SHALL trigger a quiesce callback when a claim handle is released.

#### Scenario: Releasing claim slot
- **WHEN** `release(slot)` is invoked
- **THEN** the slot is returned to the free pool and the registered `PinQuiesce` callback is invoked for each released pin.

