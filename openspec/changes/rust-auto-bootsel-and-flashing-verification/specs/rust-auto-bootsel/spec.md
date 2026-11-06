## ADDED Requirements

### Requirement: Software USB BOOTSEL Trigger
The firmware SHALL provide commands to reboot the RP2350 microcontroller into USB BOOTSEL mode without physical button presses.

#### Scenario: Serial REPL bootsel command
- **WHEN** the user sends `bootsel` or `BOOTSEL` to the CDC serial REPL on `/dev/ttyACM0`
- **THEN** the firmware writes `Rebooting into BOOTSEL mode...\r\n` and immediately invokes the bootrom reset function to enter BOOTSEL mode

#### Scenario: REST API bootsel trigger
- **WHEN** a client issues `POST /api/v1/system/bootsel`
- **THEN** the firmware returns an HTTP 200 JSON confirmation and resets the probe into BOOTSEL mode
