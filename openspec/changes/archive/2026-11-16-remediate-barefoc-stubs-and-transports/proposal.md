## Why

An audit of the native BareFOC Studio system app (`webui/barefoc.html`) identified that while the SimpleFOC Commander ASCII serial path is fully functional, several core protocols and hardware interaction modules are currently simulated stubs:
1. `FibreAdapter` simulates binary RPC connection and schema introspection instead of exchanging real binary frames over WebSocket or TCP.
2. `MultiBusAdapter` logs memory offsets to the console rather than dispatching actual I2C/SPI bus transfers.
3. The Hardware Alignment Wizard uses a mock `setInterval` JavaScript timer rather than dispatching real calibration commands (`init_foc()`) to the target hardware and parsing physical zero angle results.
4. CAN Bus transport is unrouted in the client code even though AgentLA exposes a live hardware CAN WebSocket bridge (`/api/v1/can/ws`).
5. Gate driver SPI registers are purely decorative HTML cards without read/write capability.

Remediating these gaps replaces all simulated stubs with live, functional protocol and hardware bridge implementations.

## What Changes

- **Live CAN Bus Transport**:
  - Wire `/api/v1/can/ws` into `webui/barefoc.html` supporting both SimpleFOC CAN and ODrive CAN frame formats.
  - Implement CAN ID filtering, message transmission, and telemetry frame parsing.
- **Real Binary Fibre RPC Engine**:
  - Implement client-side binary packet framing (start byte `0xAA`, header CRC8, packet payload CRC16).
  - Implement endpoint 0 JSON schema request, decompression, and client-side tree caching.
  - Support typed read/write endpoints for velocity, position, Iq current, and bus voltage.
- **Real MultiBus I2C/SPI Bridge Transport**:
  - Wire `MultiBusAdapter` to communicate through AgentLA's active bus bridges or MCP tools (`i2c_scan`, `spi_transfer`).
  - Read and write physical control table registers (Target, Control Mode, Voltage Limit, Velocity).
- **Interactive Hardware Alignment Sequence**:
  - Replace the artificial 400ms JS timer in the Hardware Alignment Wizard with an asynchronous command dispatcher that executes SimpleFOC `init_foc()` / alignment routines over the active transport.
  - Await and parse actual `zero_electric_angle`, sensor direction, and pole pair verification from target hardware responses.
- **Gate Driver Register Configuration**:
  - Wire DRV8316 and DRV8311 register inputs to transmit SPI read and write commands over the active bus or serial bridge.
- **Real Telemetry Parsing**:
  - Replace hardcoded synthetic trigonometric phase voltages (`Math.sin(theta_e) * 12.0`) with telemetry-derived values or direct measurement indicators.

## Capabilities

### New Capabilities
<!-- No new capabilities required; extending existing capability -->

### Modified Capabilities
- `native-barefoc-system-app`: Update requirements to enforce real binary Fibre RPC framing, active CAN bus transport over `/api/v1/can/ws`, hardware MultiBus I2C/SPI transfers, and non-simulated alignment command sequencing.

## Impact

- `webui/barefoc.html`: FibreAdapter, MultiBusAdapter, CAN transport, and Alignment Wizard rewritten with production logic.
- `SYSTEM_FS`: Packaged image updated with non-stubbed `barefoc.html`.
- Zero changes to core firmware binaries required; utilizes existing `/api/v1/can/ws`, `/api/v1/terminal/ws`, and bus bridge endpoints.
