## ADDED Requirements

### Requirement: Protocol-Qualified Capture Triggering
The logic analyzer capture core SHALL accept trigger qualifications from the Hardware Trigger Matrix, allowing protocol event matches to trip the capture state machine from `Capturing` to `Captured` with configurable post-trigger sample retention.

#### Scenario: Protocol trigger terminates capture
- **WHEN** capture is armed in protocol-trigger mode and the Hardware Trigger Matrix fires
- **THEN** capture engine completes post-trigger sample collection and transitions state to Captured
