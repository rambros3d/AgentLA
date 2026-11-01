# usb-i2c-tiny-usb Specification

## Purpose
TBD - created by archiving change usb-native-adapter-endpoints. Update Purpose after archive.
## Requirements
### Requirement: Native i2c-tiny-usb Protocol Handling
The firmware SHALL handle `i2c-tiny-usb` control transfer requests on Endpoint 0.

#### Scenario: Host scans I2C bus
- **WHEN** host executes `i2cdetect -y -r 1`
- **THEN** firmware executes I2C start/stop/address transactions and returns acknowledge results to the host `i2c-core` subsystem

### Requirement: Auto-Claim on First I2C Access
The system SHALL automatically claim default flexible pins GP10 (SDA) and GP11 (SCL) if host attempts an I2C transaction while pins are unassigned.

#### Scenario: Implicit first access
- **WHEN** host sends an I2C transaction before explicit WebUI routing
- **THEN** system automatically claims GP10/11 for I2C and completes the transaction

