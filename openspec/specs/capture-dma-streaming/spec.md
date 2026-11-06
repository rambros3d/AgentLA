# capture-dma-streaming Specification

## Purpose
DMA streaming and bank buffer capture engine for the high-speed logic analyzer.

## Requirements

### Requirement: Capture core state management
The logic analyzer core SHALL support arming, sampling, and stopping states with deterministic transitions.

#### Scenario: Arm and stop
- **WHEN** engine is armed and stop is requested
- **THEN** DMA and PIO state machines halt cleanly and buffers remain valid.

### Requirement: PIO DMA capture bank
The capture system SHALL stream samples via PIO DMA into allocated capture banks.

#### Scenario: No trigger 1024
- **WHEN** capture is configured for 1024 samples without trigger
- **THEN** 1024 samples are collected into the capture bank.

### Requirement: Memory overrun guard
The engine SHALL reject capture depth configurations that exceed available capture bank capacity.

#### Scenario: Oversize requested
- **WHEN** client requests capture depth greater than maximum bank capacity
- **THEN** API rejects request with 503 error.
