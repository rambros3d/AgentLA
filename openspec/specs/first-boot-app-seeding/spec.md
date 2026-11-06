# first-boot-app-seeding Specification

## Purpose
Automatically unpacks and initializes starter applications into LittleFS flash partition on first boot or when missing.

## Requirements

### Requirement: Automatic First-Boot Application Seeding
The firmware SHALL automatically unpack and write the starter `servo-master` application into the LittleFS flash partition under `/apps/servo-master/` when LittleFS is initially formatted or whenever `/apps/servo-master` is missing on boot.

#### Scenario: First boot into fresh LittleFS
- **WHEN** the device boots with an empty or newly-formatted LittleFS partition
- **THEN** firmware creates `/apps/servo-master/` and writes `app.json` and `index.html` from built-in ROM assets.

#### Scenario: Immediate availability in launcher
- **WHEN** user opens `http://10.10.10.10/apps` after flashing
- **THEN** "Servo Master" is immediately displayed under "My Apps" with launch and delete buttons without requiring internet or manual file upload.
