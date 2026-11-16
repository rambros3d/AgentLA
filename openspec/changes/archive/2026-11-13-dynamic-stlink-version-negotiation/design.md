## Context

STMicroelectronics debug tooling (STM32CubeIDE `ST-LINK_gdbserver`, STM32CubeProgrammer, STM32CubeCLT) issues command byte `0xF1` to determine the probe's firmware version. The genuine ST-Link response encodes:
- Major hardware version (V): 4 bits (e.g. 2 for ST-Link/V2-1).
- JTAG/SWD protocol version (J): 6 bits (e.g. 48).
- Mass storage / SWIM version (M): 6 bits (e.g. 35).

When users install newer STM32CubeIDE releases, the IDE may refuse to start debugging if the probe reports a version below its expected minimum (e.g. J46 or J48). Instead of forcing developers to update firmware or recompile code, `ref/hybridlink` stores this version as a configurable setting.

## Goals / Non-Goals

**Goals:**
- Provide a runtime configurable version word in `StLinkHandler` defaulting to `V2J48M35` (`0x2C23`).
- Implement string parser and formatter supporting standard ST-Link notation (`V<v>J<j>M<m>`).
- Persist user-selected versions in LittleFS at `/system/stlink_version.txt`.
- Expose REST API `GET /api/programmer/stlink/version` and `POST /api/programmer/stlink/version`.
- Provide a version selector and write-in input in the WebUI System Settings modal.

**Non-Goals:**
- Emulating ST-Link V3 multi-AP extended protocol (AgentLA emulates the widely-compatible V2-1 protocol).

## Decisions

### Decision 1: LittleFS ASCII Persistence

- **Choice**: Store the version as a plain UTF-8 text string (e.g. `"V2J48M35"`) in `/system/stlink_version.txt`.
- **Rationale**: Human-readable, easily inspected via the Bento File Manager, and simple to reset by deleting the file.

### Decision 2: Fallback to Safe Default

- **Choice**: If the file does not exist, contains whitespace, or has an invalid format, fallback gracefully to `V2J48M35` (`0x2C23`).
- **Rationale**: Guarantees probe operation even after LittleFS partition format or first boot.

## Risks / Trade-offs

- **[Risk] User enters an excessively low version (e.g. V2J15M0) and gets locked out by STM32CubeIDE**
  - *Mitigation*: The WebUI provides validated presets (V2J46M27, V2J48M35, V2J49M36) with clear labels, plus a "Reset to Default" button.
