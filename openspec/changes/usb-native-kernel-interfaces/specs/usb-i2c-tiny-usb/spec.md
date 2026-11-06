# usb-i2c-tiny-usb Specification

## Purpose
Provide a native, driverless USB-to-I2C interface conforming to the `i2c-tiny-usb` protocol supported natively by the Linux kernel (`drivers/i2c/busses/i2c-tiny-usb.c`).

## Requirements

### Requirement: Native i2c-tiny-usb Protocol Handling
The firmware SHALL handle `i2c-tiny-usb` control transfer requests on Endpoint 0.

#### Scenario: Host probes i2c-tiny-usb device
- **WHEN** host sends `CMD_ECHO` with 16-bit challenge `0x1234`
- **THEN** firmware returns the exact 16-bit value `0x1234` to confirm device presence.

#### Scenario: Host queries bus capabilities
- **WHEN** host sends `CMD_GET_FUNC`
- **THEN** firmware returns `I2C_FUNC_I2C | I2C_FUNC_SMBUS_EMUL` (0x0EFF0001).

### Requirement: I2C Data Transfers (CMD_I2C_IO)
The firmware SHALL execute I2C write and read transactions on the target bus pins and update transaction status (`STATUS_ADDRESS_ACK` or `STATUS_ADDRESS_NAK`).

#### Scenario: Host scans or accesses I2C bus
- **WHEN** host sends `CMD_I2C_IO` for address `0x48`
- **THEN** firmware generates START condition, writes address, transfers data, generates STOP condition, and records ACK/NAK status for `CMD_GET_STATUS`.
