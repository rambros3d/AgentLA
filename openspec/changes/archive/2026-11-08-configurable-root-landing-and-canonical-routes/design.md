## Context

AgentLA delivers an embedded web user interface running on an RP2350 microcontroller. When a host machine plugs in via USB-C, the probe configures an RNDIS network interface allocating the IP `10.10.10.10` (`http://agentla.local`).

Historically, the root `/` redirected to legacy `/lab` or `/apps`, while settings were only retained in volatile RAM. Since backward compatibility is not required, we are removing legacy routes and standardizing on a clean set of canonical routes (`/home`, `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, `/terminal`), with settings persisted to `/system/settings.json` on LittleFS.

## Goals / Non-Goals

**Goals:**
- Provide instant 302 redirection from `http://10.10.10.10/` to the user-configured default landing route (default: `/home`).
- Implement LittleFS configuration persistence in `firmware-rs`:
  - Load `/system/settings.json` during boot.
  - Save to `/system/settings.json` upon `POST` or `PUT` to `/api/v1/settings`.
- Streamline firmware routing table in `firmware-rs/src/web.rs` by removing deprecated routes (`/lab`, `/apps`, `/target`, `/console`, `/pin`).
- Standardize canonical routes across both firmware and frontend:
  - `/home`: Launcher and store (`apps.html`)
  - `/scope`: Scope & Logic Analyzer (`index.html#scope`)
  - `/debug`: SWD / RV-SWD / QingKe Debugger (`index.html#debug`)
  - `/adapter`: Protocol Bus Masters (`index.html#adapter`)
  - `/pins`: 30-Pin Matrix & Crossbar (`index.html#pins`)
  - `/agent`: Model Context Protocol (MCP) Console (`index.html#agent`)
  - `/terminal`: In-browser hardware serial terminal (`terminal.html`)
- Align `webui/app.js`, `webui/apps.html`, `webui/index.html`, and `webui/mock-bridge.js` to use `/home` as the default landing route and sync the settings dropdown.

**Non-Goals:**
- Supporting legacy `/lab` or `/apps` URLs (backward compatibility is explicitly out of scope).
- Altering user app LittleFS packaging or `/app/{name}/` routing.

## Decisions

### Decision 1: LittleFS JSON Configuration File (`/system/settings.json`)
- **Choice**: Store UI configuration as a compact JSON string at `/system/settings.json` within LittleFS `SYSTEM_FS`.
- **Rationale**:
  - LittleFS provides flash wear leveling and atomic file updates.
  - JSON format is human-readable and matches the payload exchanged with the frontend `/api/v1/settings` REST endpoint.
  - RAM caching in `static UI_SETTINGS` ensures `GET /` redirects remain instantaneous without incurring disk I/O on every page navigation.
- **Alternatives considered**:
  - *Raw binary EEPROM/Flash sector*: Rejected because LittleFS is already mounted and manages partition wear leveling.
  - *Browser localStorage only*: Rejected because entering `http://10.10.10.10` from a new browser tab or device would not remember the device-level default landing page.

### Decision 2: Elimination of Legacy Aliases
- **Choice**: Remove `/lab`, `/apps`, `/target`, `/console`, and `/pin`.
- **Rationale**:
  - The user explicitly requested: "backward compatibility is not needed".
  - Removing duplicate route registrations shrinks the picoserve routing state machine in firmware ROM and eliminates ambiguity across documentation and UI code.

### Decision 3: Frontend Deep-Linking Synchronization
- **Choice**: Use HTML5 `history.pushState(null, '', '/<canonical_route>')` when switching modes inside `webui/app.js`.
- **Rationale**:
  - When a user clicks between Scope, Debug, Adapter, Pins, and Agent inside the workbench, the address bar reflects the exact tool currently open.
  - Bookmarking or sharing `http://10.10.10.10/debug` immediately loads the debugger on page refresh.

## Risks / Trade-offs

- **[Risk]** Unformatted or fresh LittleFS partition on first boot does not contain `/system/settings.json`.  
  → **Mitigation**: When `/system/settings.json` cannot be opened or read, firmware falls back to factory defaults (`/home`, `system` mode, `emerald`/`forest` themes) without panicking.

- **[Risk]** LittleFS filesystem contention during settings write.  
  → **Mitigation**: The settings write acquires the global LittleFS mutex (`crate::fs::SYSTEM_FS.lock()`), performs a single buffered write, and unlocks immediately.
