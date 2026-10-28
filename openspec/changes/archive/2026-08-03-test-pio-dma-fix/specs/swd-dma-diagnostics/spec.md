## ADDED Requirements

### Requirement: DMA transfer timeout detection
The system SHALL detect when a PIO DMA transfer fails to complete within a configurable timeout period (default 100ms) and return an error code instead of blocking forever.

#### Scenario: DMA completes normally
- **WHEN** `tap_pio_dma_send_uint32` is called and the DMA transfer finishes within the timeout
- **THEN** the function returns normally and the TX FIFO has been fully consumed by the PIO SM

#### Scenario: DMA hangs and times out
- **WHEN** `tap_pio_dma_send_uint32` is called and the DMA does not complete within 100ms
- **THEN** the function aborts the DMA transfer, stores PIO SM state in diagnostic globals, and returns without blocking the main loop

#### Scenario: DMA timeout preserves diagnostic state
- **WHEN** a DMA timeout occurs
- **THEN** the following PIO state is captured: SM program counter, TX FIFO level, RX FIFO level, TXSTALL bit, execution stall bit, and the last buffer address that was being sent

### Requirement: PIO SM state dump
The system SHALL provide a CLI command (`DIAG`) that reads and prints the current PIO1 SM0 state registers over CDC0, including PC, FIFO levels, stall bits, and the diagnostic globals from the last DMA timeout (if any).

#### Scenario: DIAG command with no prior hang
- **WHEN** the user sends `DIAG\r\n` on CDC0 and no DMA timeout has occurred
- **THEN** the system prints the current PIO SM0 registers (PC, FIFO levels, stall bits) and "No prior hang detected"

#### Scenario: DIAG command after a hang
- **WHEN** the user sends `DIAG\r\n` on CDC0 and a DMA timeout was previously detected
- **THEN** the system prints the captured PIO state from the timeout event, including the buffer that was being sent

### Requirement: LED diagnostic toggle
The system SHALL toggle the onboard LED (GPIO25) at key points in the SWD flow to allow oscilloscope-based debugging of hangs: LED ON before `tap_pio_dma_send_uint32`, LED OFF after it returns.

#### Scenario: LED reflects DMA progress
- **WHEN** an SWD operation is initiated
- **THEN** the LED turns ON just before DMA start and turns OFF just after DMA completes (or times out)

#### Scenario: LED stuck ON indicates hang
- **WHEN** the LED remains ON for more than 1 second
- **THEN** the PIO DMA transfer is stuck and the DIAG command should be used to inspect PIO state
