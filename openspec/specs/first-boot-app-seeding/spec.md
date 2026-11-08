# first-boot-app-seeding Specification

## Purpose
Automatically unpacks and initializes starter applications into LittleFS flash partition on first boot or when missing.

## Requirements

### Requirement: Automatic First-Boot Application Seeding
The firmware SHALL initialize default application directories and manifests under `/apps/` (`servo-master`, `terminal`, and `can-sniffer`) in LittleFS without embedding large static HTML string constants in firmware ROM flash.

#### Scenario: First boot into fresh LittleFS
- **WHEN** the device boots with an empty or newly-formatted LittleFS partition
- **THEN** firmware creates `/apps/servo-master/`, `/apps/terminal/`, and `/apps/can-sniffer/` and writes their manifest `app.json` files without hardcoding static HTML bodies into ROM flash.

#### Scenario: Immediate availability in launcher
- **WHEN** user opens `http://10.10.10.10/apps` after flashing
- **THEN** seeded applications are listed under "My Apps" and their rich web UI interfaces are served from the LittleFS WebUI partition (`SYSTEM_FS`).
