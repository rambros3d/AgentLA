## ADDED Requirements

### Requirement: Double-buffered concurrent flashing
The system SHALL support double-buffered concurrent page programming during flash algorithm execution to overlap transport streaming with on-chip flash burning.

#### Scenario: Ping-pong buffer execution
- **WHEN** flashing a target whose algorithm blob specifies sufficient target SRAM for two page buffers
- **THEN** the system alternates between Buffer 0 and Buffer 1, streaming chunk $N+1$ over SWD while the target CPU is actively executing ProgramPage on chunk $N$

#### Scenario: Fallback for low-RAM targets
- **WHEN** target SRAM is insufficient for two full page buffers
- **THEN** the system automatically falls back to synchronous single-buffer flashing without failing
