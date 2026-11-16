## MODIFIED Requirements

### Requirement: BareBoot I3C Bootloader Flashing
The system SHALL flash compiled BareBoot bootloader images into target microcontroller flash memory starting at the target flash base address (0x08000000 for STM32/CH32V, 0x10000000 for RP2350) via the onboard hardware programmer.

#### Scenario: Flash bootloader sector
- **WHEN** user initiates bootloader flashing with a BareBoot binary image
- **THEN** sector 0 is unlocked, erased, and programmed with verification
- **AND** a software reset is issued via CoreSight DHCSR SYSRESETREQ or target reset line

### Requirement: Bootloader Host Bus Verification
The system SHALL verify that BareBoot initializes its host communication bus and responds to bus enumeration at target address 0x40 (I2C), dynamic address assignment (I3C), CAN-FD node ID, or UART telemetry.

#### Scenario: Katapult connect handshake
- **WHEN** the host sends a Katapult CONNECT frame (command 0x11) to the bootloader target
- **THEN** the bootloader responds with ACK containing protocol version, MCU family, and application base address
