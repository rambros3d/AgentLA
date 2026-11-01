## ADDED Requirements

### Requirement: Circular Pre-Trigger and Post-Trigger Buffer Allocation
The logic capture architecture SHALL allocate a 64 KB circular DMA ring buffer for pre-trigger trace history and a 384 KB linear buffer for post-trigger acquisition within the RP2350's 520 KB internal SRAM.

#### Scenario: Continuous pre-trigger overwrite before trigger event
- **WHEN** logic capture is armed and running before a trigger condition occurs
- **THEN** DMA Channel 0 continuously records new samples into the 64 KB circular ring, overwriting the oldest samples without host CPU intervention

### Requirement: Hardware PIO Trigger Qualification and USB Dump
A companion PIO state machine SHALL evaluate trigger patterns (rising edge, falling edge, exact pattern, or pulse width) in 1 clock cycle and fire a hardware IRQ that activates the 384 KB post-trigger DMA channel.

#### Scenario: Single-shot capture completion and USB drain
- **WHEN** the post-trigger DMA channel completes filling the 384 KB linear buffer
- **THEN** the firmware halts capture and transfers the combined 448 KB buffer (64 KB pre-trigger + 384 KB post-trigger) to the host PC over USB 1.1 in under 450 ms
