## ADDED Requirements

### Requirement: Target debug session abstraction

The `agentla-programmer` crate SHALL expose a `DetachPolicy` type with `Leave`, `Halt`, and `Run`
variants, and a `CortexM::teardown` operation that applies a policy to the target debug state.

`teardown` SHALL restore the `DEMCR` value captured during attach, then apply the policy to
`DHCSR`. It SHALL be callable on any target state and SHALL return `Result` for diagnostics while
remaining safe to ignore.

The firmware session layer SHALL invoke teardown from the session's `Drop` implementation, before
transport deinitialization and pin tristate, so that every early return and error path restores
target debug state.

#### Scenario: Crate compiles for host tests

- **WHEN** building `agentla-programmer` for `x86_64-unknown-linux-gnu` with the mock transport
- **THEN** `DetachPolicy` and `CortexM::teardown` are available and exercised by unit tests without
  any `rp-pac` or Cortex-M runtime dependency

#### Scenario: Teardown preserves a prior TRCENA

- **WHEN** the snapshotted `DEMCR` had `TRCENA` set and the detach policy is `Run`
- **THEN** teardown writes a value with `TRCENA` set and `VC_CORERESET` clear

#### Scenario: Teardown with no snapshot

- **WHEN** the DEMCR snapshot read failed during attach
- **THEN** teardown writes `0` to `DEMCR`, disarming all vector catch