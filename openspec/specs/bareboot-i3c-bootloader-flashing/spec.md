# bareboot-i3c-bootloader-flashing Specification

## Purpose
TBD - created by archiving change stm32h503-swd-dormant-and-bareboot-flash. Update Purpose after archive.
## Requirements
### Requirement: BareBoot I3C Bootloader Flashing
The system SHALL flash the compiled BareBoot I3C bootloader image into STM32H503 flash memory starting at base address `0x08000000` via the MCP `programmer_flash` tool.

#### Scenario: Flash 8KB bootloader sector
- **WHEN** user initiates bootloader flashing with the BareBoot binary image
- **THEN** sector 0 (0x08000000 - 0x08001FFF) is unlocked, erased, and programmed with 128-bit quad-word verification
- **AND** a software reset is issued via CoreSight DHCSR SYSRESETREQ

### Requirement: Bootloader Host Bus Verification
The system SHALL verify that BareBoot initializes its host communication bus and responds to bus enumeration at target address `0x40` (I2C) or dynamic address assignment (I3C).

#### Scenario: Katapult connect handshake
- **WHEN** the host sends a Katapult CONNECT frame (command 0x11) to the bootloader target
- **THEN** the bootloader responds with ACK containing protocol version, MCU family, and application base address

