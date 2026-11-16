## 1. Fibre Protocol Implementation

- [x] 1.1 Implement binary `FibreClient` in `webui/barefoc.html` with packet framing (`0xAA` sync, CRC8 header, CRC16 payload).
- [x] 1.2 Implement endpoint 0 schema request, JSON schema parsing, and typed endpoint read/write helpers.
- [x] 1.3 Wire `FibreAdapter` to exchange real binary frames over WebSocket or TCP proxy and populate telemetry.

## 2. CAN Bus Transport Integration

- [x] 2.1 Wire CAN Bus selection to AgentLA's active WebSocket bridge `/api/v1/can/ws`.
- [x] 2.2 Implement CAN frame encoding/decoding for 11-bit SimpleFOC CAN and ODrive CAN (node ID + command ID).
- [x] 2.3 Add client-side CAN node ID filtering and telemetry frame ingestion.

## 3. MultiBus Bridge Integration

- [x] 3.1 Connect `MultiBusAdapter` to AgentLA's active I2C/SPI bridge endpoints (`/api/v1/bridge/i2c` and `/api/v1/bridge/spi`).
- [x] 3.2 Wire register read and write operations for target control table registers (Target, Control Mode, Voltage Limit, Velocity).

## 4. Hardware Alignment Wizard & Gate Driver Wiring

- [x] 4.1 Replace mock `setInterval` in Alignment Wizard with asynchronous SimpleFOC `init_foc()` command dispatcher.
- [x] 4.2 Parse physical `zero_electric_angle` and direction responses to complete motor alignment calibration.
- [x] 4.3 Connect gate driver SPI register read/write triggers for DRV8316 and DRV8311.
- [x] 4.4 Replace synthetic phase voltages with real telemetry-derived values.

## 5. System Packaging & Hardware Verification

- [x] 5.1 Package updated `webui/barefoc.html` into `SYSTEM_FS` image and tarball using `tools/pack_webui.py`.
- [x] 5.2 Deploy WebUI update to AgentLA probe at `10.10.10.10` via `POST /api/v1/system/webui/update`.
- [x] 5.3 Verify `/motor` loads cleanly with zero console syntax errors and live protocol adapters.
