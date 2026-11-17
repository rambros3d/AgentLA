## ADDED Requirements

### Requirement: Native In-ROM System Asset
The system SHALL provide `webui/sensor.html` as a canonical system asset packaged into the `SYSTEM_FS` partition.

#### Scenario: Serving sensor asset from SYSTEM_FS
- **WHEN** client requests `GET /sensor.html` or `GET /sensor`
- **THEN** server streams the asset from `SYSTEM_FS` using pre-compressed gzip if supported by client

### Requirement: Firmware Route Registration
The system SHALL route `/sensor` to `SystemWebFile::new("sensor.html", "text/html")` and provide HTTP redirects for `/sensors` and `/bus`.

#### Scenario: Accessing canonical sensor route
- **WHEN** user navigates to `/sensor`
- **THEN** server responds with HTTP 200 and serves Sensor Studio

#### Scenario: Accessing sensor alias routes
- **WHEN** user navigates to `/sensors` or `/bus`
- **THEN** server responds with HTTP 301 redirect to `/sensor`

### Requirement: Dual-Bus Control Interface
The system SHALL support both I2C and SPI bus modes selectable via UI tab.

#### Scenario: I2C scan execution
- **WHEN** user clicks "Scan Bus" in I2C mode
- **THEN** system queries addresses `0x08..0x77` via `/api/v1/bridge/i2c/scan` and highlights responsive addresses in the radar grid

#### Scenario: SPI device probing
- **WHEN** user selects an active CS pin and clicks "Probe SPI Device"
- **THEN** system queries JEDEC ID and standard register signatures via `/api/v1/bridge/spi/transfer`

### Requirement: Sensor Profile Registry & Visualizers
The system SHALL fingerprint responsive addresses against built-in profiles and render specialized visualizers.

#### Scenario: IMU detected
- **WHEN** device at `0x68` responds with WHO_AM_I `0x68`
- **THEN** system displays InvenSense MPU-6050 card and loads the 3D attitude orientation visualizer

#### Scenario: Magnetic angle sensor detected
- **WHEN** device at `0x35` responds with TI signature `0x5449`
- **THEN** system displays TI TMAG5273 card and loads the rotary angle compass dial

### Requirement: Top-Rail Navigation and Dock Integration
The system SHALL display the `Sensor` instrument in the top navigation rail across all system pages and in the dock using the Solar Bold Duotone Radar icon.

#### Scenario: Launching from dock
- **WHEN** user clicks the Radar icon in the dock
- **THEN** system navigates directly to `/sensor`
