## ADDED Requirements

### Requirement: Co-located Browser Simulator for WebUI
The `webui/` directory SHALL contain `mock-bridge.js` providing in-browser hardware emulation and synthetic waveform generation, allowing `webui/index.html` and `webui/apps.html` to run in simulated demo mode on any static host or local server without missing dependency 404 errors.

#### Scenario: Running WebUI in demo mode standalone
- **WHEN** a user or developer opens `webui/index.html?demo=1` or runs a static server on a non-standard port
- **THEN** the page automatically detects demo mode, loads `mock-bridge.js` from the same directory, initializes the simulated 30-pin register state machine, and begins rendering synthetic logic analyzer waveforms

### Requirement: Exclusion of Simulation Logic from Firmware LittleFS Packaging
The firmware asset packaging pipeline (`tools/pack_webui.py`) MUST explicitly exclude `mock-bridge.js` and demo board assets from the generated LittleFS filesystem image (`build/webui.img`), ensuring zero flash memory overhead on the physical RP2350 target.

#### Scenario: Packaging WebUI into LittleFS binary
- **WHEN** `python3 tools/pack_webui.py` is executed to build `build/webui.img`
- **THEN** `mock-bridge.js` is excluded from the discovery list, and the resulting LittleFS filesystem contains only production embedded assets
