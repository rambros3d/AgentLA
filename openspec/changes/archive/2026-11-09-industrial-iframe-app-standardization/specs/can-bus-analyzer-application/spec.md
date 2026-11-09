## MODIFIED Requirements

### Requirement: Manual and Periodic Frame Transmission
The web application SHALL allow the user to construct and inject standard or extended CAN frames with arbitrary ID, DLC, and payload bytes, supporting both single-shot transmission and periodic interval repeating. The TX Frame Injector SHALL be positioned in a fixed top Bento card located directly beneath the filtering toolbar, ensuring persistent visibility regardless of packet table scroll depth.

#### Scenario: Inject single CAN frame
- **WHEN** user specifies ID `0x7DF` with DLC `8` and payload `02 01 0D 55 55 55 55 55` and clicks "Send Frame"
- **THEN** the application transmits the frame payload across the WebSocket bridge to the hardware transceiver.

#### Scenario: Persistent top visibility during high-rate capture
- **WHEN** hundreds of CAN frames stream into the packet table
- **THEN** the TX Frame Injector card remains fixed at the top of the interface and accessible without scrolling.

## ADDED Requirements

### Requirement: CAN Bus Analyzer Industrial Styling and Embedded Viewport
The CAN Bus Analyzer application SHALL apply the Industrial Design System (`data-style="industrial"`, `data-theme="nothing"` / `industrial-light`, NDot57 typography, dot-grid canvas, and zero emojis), and SHALL provide at least 72px bottom clearance padding on the frame log table to avoid occlusion by the host system dock when embedded.

#### Scenario: Embedded iframe theme synchronization
- **WHEN** the CAN Bus Analyzer loads or receives a theme change event from the host shell
- **THEN** the application updates its theme attributes and displays industrial-styled Bento cards with 72px bottom clearance above the dock.
