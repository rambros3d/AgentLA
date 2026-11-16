## 1. App Package Foundation

- [x] 1.1 Create `apps/barefoc/app.json` with metadata, permissions (`uart`, `can`, `gpio_flexible`), and version info
- [x] 1.2 Create `apps/barefoc/icon.svg` featuring technical FOC vector branding
- [x] 1.3 Register `barefoc` in `apps/catalog.json` with categories, size estimate, and package link

## 2. Protocol Engine & Hardware Bridge Client

- [x] 2.1 Implement `FibreAdapter` in JavaScript (framing, endpoint 0 JSON schema parser, read/write primitives)
- [x] 2.2 Implement `CommanderAdapter` in JavaScript (SimpleFOC ASCII formatting, response parser, monitor stream)
- [x] 2.3 Implement `MultiBusAdapter` in JavaScript (descriptor table mapping, register read/write)
- [x] 2.4 Wire transport connections to AgentLA UART WebSocket (`/api/v1/terminal/ws`), CAN WebSocket (`/api/v1/can/ws`), and WebSerial
- [x] 2.5 Implement virtual motor simulator mode (`?demo=1`) for standalone testing

## 3. UI Layout & View Modules

- [x] 3.1 Construct top header and view navigation rail with AgentLA theme synchronization and emergency safe-state pill
- [x] 3.2 Build View 1 (Live Studio & Multi-Mode Dashboard): Motion modes, torque modes, target dial, and telemetry HUD
- [x] 3.3 Build View 2 (Cascaded Controller Tuning): Angle P, Velocity PID + LPF, Current Iq/Id PID + LPF, and modulation mode selectors
- [x] 3.4 Build View 3 (Real-Time Oscilloscope): HTML5 Canvas 2D ring buffer, dual-trace plotting, step response benchmarks, and CSV export
- [x] 3.5 Build View 4 (Hardware Wizard): Motor parameters, smart driver registers (DRV8316/DRV8311), sensor setup, and electrical zero search
- [x] 3.6 Build View 5 (Low-Level Console): Direct terminal, Fibre RPC object tree explorer, MultiBus register table, and error clearing

## 4. Packaging, Verification & Integration

- [x] 4.1 Assemble complete, self-contained `apps/barefoc/index.html` referencing shared system assets
- [x] 4.2 Package `apps/packages/barefoc.tar` as a POSIX ustar archive and verify package size is under 768 KB
- [x] 4.3 Validate LittleFS unpacking flow and verify demo mode and browser rendering
