# bareboot-manager-application Specification

## Purpose
TBD - created by archiving change bareboot-manager-app. Update Purpose after archive.
## Requirements
### Requirement: BareBoot Manager Desktop UI Integration
The system SHALL provide a native web application in `apps/bareboot/` registered in `apps/catalog.json` with an industrial theme, full-bleed viewport, bottom-anchored dock, and zero dot indicators.

#### Scenario: Launch BareBoot Manager from Desktop Shell
- **WHEN** user navigates to `/apps/bareboot/` or launches it from the AgentLA OS dock
- **THEN** the BareBoot Workbench loads with tab navigation for Bootloader Flasher, Configuration, and Firmware Updater
- **AND** the AgentLA OS top bar and floating glass dock synchronize without floating dot indicators

### Requirement: Hardware Programmer Bootloader Bootstrap
The system SHALL allow users to detect connected target microcontrollers and program BareBoot bootloader images into target flash memory via AgentLA's onboard programmer.

#### Scenario: Auto-detect target and flash bootloader
- **WHEN** user connects a target MCU to programmer pins and clicks Detect Target
- **THEN** the system queries `/api/v1/programmer/target` and displays MCU family, IDCODE, and flash size
- **AND** when user selects a bootloader binary and clicks Flash Bootloader, the system programs the binary to flash base and resets the core

### Requirement: BareBoot Configuration Profile Generator
The system SHALL provide an interactive configuration editor to view, edit, and export `bareboot.toml` configuration files.

#### Scenario: Configure transports and boot window
- **WHEN** user modifies system clock, boot window timeout, active transports (UART, I2C, I3C, CAN), and target address
- **THEN** the system generates valid TOML matching BareBoot specifications and allows downloading or saving to LittleFS

### Requirement: Handsfree Bootloader Entry
The system SHALL transmit handsfree entry tokens across selected communication buses to trigger target application reboot into BareBoot.

#### Scenario: Handsfree reboot over serial UART
- **WHEN** user initiates handsfree entry with UART selected
- **THEN** the system transmits the string `~ \x1c Request Serial Bootloader!! ~` over `/api/v1/terminal/ws`

#### Scenario: Handsfree reboot over I2C or I3C
- **WHEN** user initiates handsfree entry with I2C or I3C selected
- **THEN** the system issues command byte `0xF0` to the target bus address

#### Scenario: Handsfree reboot over CAN
- **WHEN** user initiates handsfree entry with CAN selected
- **THEN** the system transmits an admin reboot frame on CAN ID `0x3F0` with opcode `0x02` over `/api/v1/can/ws`

### Requirement: Katapult Application Firmware Update Runner
The system SHALL execute Katapult protocol handshake and block streaming to update user application firmware over UART, I2C, I3C, and CAN.

#### Scenario: Successful application firmware upload
- **WHEN** user connects to an active bootloader and drops an application binary
- **THEN** the system verifies `CONNECT` handshake parameters, slices the image into blocks, and transmits `RX_BLOCK` frames with CCITT CRC16 verification
- **AND** the UI updates a real-time progress bar, block counter, and throughput metrics until `RX_EOF` and `COMPLETE` finish successfully

