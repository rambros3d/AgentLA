## ADDED Requirements

### Requirement: LittleFS Flash Partition Allocation
The firmware SHALL allocate a dedicated LittleFS flash partition starting after the hardware profiles storage block (`0x0019_0000` / 1.6MB offset) up to the boot counter sector (`Flash End - 16KB`), scaling storage capacity based on total on-board flash (minimum 2.38 MB on 4MB flash; minimum 14.38 MB on 16MB flash).

#### Scenario: LittleFS partition mounts on boot
- **WHEN** the RP2350 microcontroller boots
- **THEN** the firmware initializes and mounts the LittleFS partition at `/apps` without modifying the factory ROM fsdata or hardware profile flash sectors.

#### Scenario: First-boot auto-formatting
- **WHEN** LittleFS fails to mount due to blank or unformatted flash
- **THEN** the firmware automatically formats the LittleFS partition, creates the `/apps` directory, and mounts successfully.

### Requirement: Storage Quota and Utilization Reporting
The firmware SHALL compute and report total partition capacity, currently consumed bytes, and remaining free bytes.

#### Scenario: Querying flash storage usage
- **WHEN** a client makes an HTTP GET request to `/api/v1/apps/storage`
- **THEN** the response returns a JSON payload containing `total_bytes`, `used_bytes`, and `free_bytes`.

### Requirement: Multi-Tenant App Directory Isolation
The firmware SHALL store all installed user applications in distinct subdirectories under `/apps/{app-name}/` within LittleFS.

#### Scenario: Listing installed applications
- **WHEN** a client queries `/api/v1/apps`
- **THEN** the firmware inspects `/apps/` and returns an array of installed app objects containing their names, versions, sizes, and metadata.

### Requirement: App Removal and Space Reclaim
The system SHALL support recursive deletion of an app directory and immediately reclaim the freed flash blocks.

#### Scenario: Deleting an installed app
- **WHEN** a client sends an HTTP DELETE request to `/api/v1/apps/{app-name}`
- **THEN** the firmware deletes all files and directories under `/apps/{app-name}/` and returns HTTP 200 OK.
