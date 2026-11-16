# stlink-version-configuration Specification

## Purpose
TBD - created by archiving change dynamic-stlink-version-negotiation. Update Purpose after archive.
## Requirements
### Requirement: ST-Link Version Representation and Formatting
The system SHALL support parsing and formatting ST-Link version strings matching the format `V<v>J<j>M<m>` (and raw 16-bit word `(v << 12) | (j << 6) | m`).

#### Scenario: Parse valid version string
- **WHEN** parsing string `"V2J48M35"`
- **THEN** system returns version word `0x2C23` (v=2, j=48, m=35)

#### Scenario: Format version word to string
- **WHEN** formatting word `0x2C23`
- **THEN** system returns string `"V2J48M35"`

### Requirement: Version Configuration REST API
The system SHALL provide REST endpoints to query and configure the reported ST-Link version with persistence in LittleFS.

#### Scenario: Query current reported version
- **WHEN** client issues `GET /api/programmer/stlink/version`
- **THEN** system returns JSON containing `version`, `raw`, and `default`

#### Scenario: Update reported version
- **WHEN** client issues `POST /api/programmer/stlink/version` with `{"version": "V2J49M36"}`
- **THEN** system validates, writes to `/system/stlink_version.txt`, updates the active `StLinkHandler`, and returns HTTP 200

