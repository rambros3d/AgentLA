## ADDED Requirements

### Requirement: Developer Soft BOOTSEL Reboot
The system SHALL expose an HTTP endpoint `POST /api/v1/system/bootsel` that invokes `embassy_rp::rom_data::reset_to_usb_boot(0, 0)` on the RP2350 target, placing the board into USB mass storage bootloader mode without requiring physical BOOTSEL button presses or power cycling.

#### Scenario: Executing soft bootsel reboot
- **WHEN** an authorized client sends `POST /api/v1/system/bootsel`
- **THEN** the firmware flushes pending logs and reboots the RP2350 core into the USB ROM bootloader mode.

### Requirement: Developer LittleFS Partition Formatting
The system SHALL expose an HTTP endpoint `POST /api/v1/system/fs/format` accepting a JSON payload specifying `"partition": "system"` or `"partition": "user"` to reformat the respective LittleFS flash area in place.

#### Scenario: Formatting LittleFS system partition
- **WHEN** a client sends `POST /api/v1/system/fs/format` with `{"partition": "system"}`
- **THEN** the firmware unmounts `SYSTEM_FS`, erases the flash sectors from `0x10270000` to `0x102F0000`, initializes a blank LittleFS structure, and returns HTTP 200 OK.
