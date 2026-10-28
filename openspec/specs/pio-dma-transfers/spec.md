# pio-dma-transfers Specification

## Purpose
TBD - created by archiving change rewrite-swd-pio-engine. Update Purpose after archive.
## Requirements
### Requirement: DMA-based PIO FIFO transfers

SWD data transfers between CPU memory and PIO state machine shall use DMA for zero-copy, non-blocking operation.

#### Scenario: Send data to PIO TX FIFO via DMA
- **WHEN** the CPU needs to send a 32-bit word to the PIO
- **THEN** the DMA channel shall transfer the word from memory to `pio->txf[sm]`
- **AND** the CPU shall not be blocked during the transfer

#### Scenario: Receive data from PIO RX FIFO via DMA
- **WHEN** the CPU needs to read a 32-bit word from the PIO
- **THEN** the DMA channel shall transfer the word from `pio->rxf[sm]` to memory
- **AND** the CPU shall be notified when the transfer completes

#### Scenario: Simultaneous send and receive
- **WHEN** an SWD transfer requires both sending a request and receiving data
- **THEN** two DMA channels shall operate simultaneously
- **AND** the send DMA shall pace on PIO TX DREQ
- **AND** the receive DMA shall pace on PIO RX DREQ

