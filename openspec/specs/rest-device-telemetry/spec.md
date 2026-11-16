# rest-device-telemetry Specification

## Purpose
Provides RESTful hardware telemetry, power management, and probe control endpoints for AgentLA.
## Requirements
### Requirement: Probe Telemetry Endpoint
The HTTP server SHALL provide `GET /api/v1/device/telemetry` returning target voltage rail measurement (VTARGET in millivolts via on-board ADC), target current draw (when supported by board hardware), MCU core temperature, and free heap/SRAM statistics.

#### Scenario: Querying device telemetry on board with current sensing
- **WHEN** client sends `GET /api/v1/device/telemetry` on a board featuring `target_current_sense: true`
- **THEN** server samples ADC lines and returns HTTP 200 with `{ "vtarget_mv": 3315, "itarget_ma": 14, "temp_c": 35.4, "free_sram_bytes": 18200 }`

#### Scenario: Querying device telemetry on board without current sensing
- **WHEN** client sends `GET /api/v1/device/telemetry` on a board featuring `target_current_sense: false`
- **THEN** server returns HTTP 200 with `itarget_ma` set to 0 and does not sample or trip on external user pin ADC1

### Requirement: Target Power Switching
The HTTP server SHALL provide `POST /api/v1/device/power` to switch target VTARGET power rail on or off when supported by hardware.

#### Scenario: Powering target board on
- **WHEN** client sends `POST /api/v1/device/power` with `{ "power": "on" }` on hardware with `target_power_control: true`
- **THEN** server enables power switch FET and returns HTTP 200 with `{ "power": "on", "status": "ok" }`

#### Scenario: Attempting power switch on unsupported hardware
- **WHEN** client sends `POST /api/v1/device/power` on hardware with `target_power_control: false`
- **THEN** server returns HTTP 400 or HTTP 422 with `{ "status": "error", "message": "Target power control not supported on active board" }`

### Requirement: Device Reset and Bootloader Jump
The HTTP server SHALL provide `POST /api/v1/device/reboot` and `POST /api/v1/device/bootsel` to initiate soft watchdog reset or jump directly to the RP2040/RP2350 USB mass-storage bootloader.

#### Scenario: Triggering BOOTSEL firmware update mode
- **WHEN** client sends `POST /api/v1/device/bootsel`
- **THEN** server responds with HTTP 200 `{ "status": "rebooting_to_bootsel" }` and reboots MCU into ROM USB UF2 flasher mode.

