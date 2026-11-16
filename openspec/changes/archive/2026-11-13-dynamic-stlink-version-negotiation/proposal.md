## Why

STM32CubeIDE (`ST-LINK_gdbserver`), STM32CubeProgrammer, and STM32CubeCLT inspect the ST-Link firmware version during initial connection handshake (`0xF1`). When a host tool is updated, newer versions often refuse older probes (*"Firmware upgrade required"*) or warn about update availability. Currently, AgentLA hardcodes `V2J48M35`, which prevents users from updating the reported version when using newer or older toolchains without recompiling the firmware.

Adapting `hybridlink`'s dynamic version configuration architecture allows AgentLA to store and update the reported version at runtime (persisted in LittleFS, configurable via WebUI, REST API, or host auto-negotiation scripts) so that any version of STM32CubeIDE connects seamlessly.

## What Changes

- **Configurable Version Architecture**: Support runtime configuration of the reported ST-Link version halfword (format `(V << 12) | (J << 6) | M`, default `V2J48M35` / `0x2C23`).
- **Persistence**: Store custom reported version strings in LittleFS at `/system/stlink_version.txt`.
- **ST-Link F1 Command Update**: Update `StLinkHandler` to dynamically return the configured version instead of a fixed constant.
- **REST API**: Provide `GET /api/programmer/stlink/version` and `POST /api/programmer/stlink/version` to query and configure the reported version on the fly.
- **WebUI Control**: Add an ST-Link Version selector and custom version input in the WebUI System Settings modal.

## Capabilities

### New Capabilities
- `stlink-version-configuration`: Runtime configuration, formatting, validation, and LittleFS persistence of ST-Link `V2JxxMxx` version identifiers with REST API controls.

### Modified Capabilities
- `stlink-probe-endpoint`: Update `F1` version command handling to return the dynamically configured version halfword.

## Impact

- **Crates**:
  - `crates/agentla-programmer`: Add version parsing/formatting functions (`parse_version`, `format_version`) and mutable version state in `StLinkHandler`.
  - `firmware-rs`: Load version from `/system/stlink_version.txt` on startup, expose REST endpoints in `rest.rs` and `web.rs`, and update UI in `webui/index.html` and `webui/app.js`.
- **Compatibility**: Allows instant resolution of STM32CubeIDE version lockouts across tool versions J32 through J55+ without reflashing AgentLA.
