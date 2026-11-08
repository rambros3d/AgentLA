## Why

When users connect to AgentLA over USB Ethernet at `http://10.10.10.10` or `http://agentla.local`, they need a fast, intuitive entry point that matches their primary workflow—whether they want a comprehensive App Launcher (`/home`), a dedicated 100 MS/s Logic Analyzer (`/scope`), a Multi-Arch Debugger (`/debug`), Protocol Bus Masters (`/adapter`), Pin Matrix (`/pins`), an MCP Agent Console (`/agent`), or Serial Terminal (`/terminal`).

Currently, the landing route settings are held only in RAM and reset upon hardware reboot, while legacy routes (such as `/lab` and `/apps`) and inconsistent singular/plural names (such as `/pin` vs `/pins`) cause confusion. Since backward compatibility is explicitly not required, we can cleanly purge legacy aliases, establish a clean set of canonical routes, and persist settings directly to LittleFS (`/system/settings.json`).

## What Changes

- **Canonical Route Architecture**:
  - Establish the official set of system routes:
    - `/home`: App Launcher, Catalog, and System Dashboard (`apps.html`) (Default landing route).
    - `/scope`: 100 MS/s Logic Analyzer and Mixed-Signal Oscilloscope (`index.html#scope`).
    - `/debug`: SWD / RV-SWD / QingKe Flasher and Hardware Debugger (`index.html#debug`).
    - `/adapter`: I2C, SPI, I3C, UART, RS-485, and CAN Bus Masters (`index.html#adapter`).
    - `/pins`: 30-Pin Crossbar and Dynamic Pin Arbiter Matrix (`index.html#pins`).
    - `/agent`: Native Model Context Protocol (MCP) Console & REPL (`index.html#agent`).
    - `/terminal`: In-Browser Hardware Serial Terminal (`terminal.html`).
- **Remove Legacy Routes (BREAKING)**:
  - Deprecate and remove legacy aliases (`/lab`, `/apps`, `/target`, `/console`, `/pin`).
- **Root 302 Redirection**:
  - `GET /` redirects via HTTP 302 to the configured `default_home_route` (default: `/home`).
- **LittleFS Settings Persistence**:
  - Store and load UI configuration (including `default_home_route`) to/from `/system/settings.json` on LittleFS.
  - On device boot, read `/system/settings.json` to initialize `UI_SETTINGS`.
  - On `POST /api/v1/settings` or `PUT /api/v1/settings`, atomically update in-memory state and persist to LittleFS.
- **Frontend Deep-Linking & History Normalization**:
  - Update `webui/app.js` routing logic and `history.pushState` to match the canonical routes (`/scope`, `/debug`, `/adapter`, `/pins`, `/agent`).
  - Standardize Settings Dropdown in `webui/index.html` and `webui/apps.html`.
  - Update `webui/mock-bridge.js` to persist simulated settings to `localStorage`.

## Capabilities

### Modified Capabilities
- `configurable-home-redirect`: Updates root 302 redirection to default to `/home`, eliminates `/lab` legacy fallback, adds LittleFS persistence (`/system/settings.json`), and updates the Settings dropdown to reflect canonical routes.
- `url-routing-engine`: Removes legacy routes (`/lab`, `/apps`, `/target`, `/console`, `/pin`) in favor of canonical routes (`/home`, `/scope`, `/debug`, `/adapter`, `/pins`, `/agent`, `/terminal`).

## Impact

- **Firmware (`firmware-rs`)**:
  - `firmware-rs/src/web.rs`: Picoserve routing table streamlined to canonical routes.
  - `firmware-rs/src/rest.rs`: Settings endpoint reads from and writes to `/system/settings.json` on LittleFS.
  - `firmware-rs/src/main.rs`: Boot sequence loads settings on startup.
- **WebUI (`webui/`)**:
  - `webui/app.js`, `webui/apps.html`, `webui/index.html`: Harmonized route maps and dropdown options.
  - `webui/mock-bridge.js`: Default landing route changed from `/lab` to `/home`.
- **Breaking Changes**:
  - Requests to `/lab` and `/apps` will return 404; clients must use `/home`, `/scope`, etc.
