## ADDED Requirements

### Requirement: Probe Telemetry Endpoint
The HTTP server SHALL provide `GET /api/v1/device/telemetry` returning target voltage rail measurement (VTARGET in millivolts via on-board ADC), MCU core temperature, and free heap/SRAM statistics.

#### Scenario: Querying device telemetry
- **WHEN** client sends `GET /api/v1/device/telemetry`
- **THEN** server samples ADC lines and returns HTTP 200 with `{ "vtarget_mv": 3315, "temp_c": 35.4, "free_sram_bytes": 18200 }`.

### Requirement: Target Power Switching
The HTTP server SHALL provide `POST /api/v1/device/power` to switch target VTARGET power rail on or off.

#### Scenario: Powering target board on
- **WHEN** client sends `POST /api/v1/device/power` with `{ "power": "on" }`
- **THEN** server enables power switch FET and returns HTTP 200 with `{ "power": "on", "status": "ok" }`.

### Requirement: Device Reset and Bootloader Jump
The HTTP server SHALL provide `POST /api/v1/device/reboot` and `POST /api/v1/device/bootsel` to initiate soft watchdog reset or jump directly to the RP2040/RP2350 USB mass-storage bootloader.

#### Scenario: Triggering BOOTSEL firmware update mode
- **WHEN** client sends `POST /api/v1/device/bootsel`
- **THEN** server responds with HTTP 200 `{ "status": "rebooting_to_bootsel" }` and reboots MCU into ROM USB UF2 flasher mode.
