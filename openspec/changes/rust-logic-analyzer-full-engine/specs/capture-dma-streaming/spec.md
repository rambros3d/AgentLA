## ADDED Requirements

### Requirement: Capture core state management
The firmware SHALL expose capture state transitions Idle -> Armed -> Capturing -> Captured, Stopped and Error with required safety `StopCapture` API.

#### Scenario: Arm and stop
- **WHEN** arm is requested on idle and then stopped
- **THEN** state reaches Stopped and pin span released

### Requirement: PIO DMA capture bank 
The firmware SHALL allocate a contiguous static bank sized according to the trivial `CaptureMode` and use DMA fill with state-completion interrupt notification.

#### Scenario: No trigger 1024
- **WHEN** requested burst 1024 with width=4 pins
- **THEN** capture reaches Captured on completion and report sample count

### Requirement: Memory overrun guard
The bank SHALL Reserve <=416KB or report HTTP 503 overrun error if asked.

#### Scenario: Oversize requested
- **WHEN** client requests > max bank cap
- **THEN** API rejects with 503
