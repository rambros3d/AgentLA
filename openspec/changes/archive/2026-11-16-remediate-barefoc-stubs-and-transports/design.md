## Context

BareFOC Studio is currently installed as a native in-ROM system app at `webui/barefoc.html` and served at `/motor`. A code audit identified that while SimpleFOC Commander ASCII over UART is functioning, the Fibre RPC client, MultiBus client, CAN bus transport, and Alignment Wizard are implemented as simulated stubs (mock console outputs and artificial JavaScript `setInterval` timers).

This design details the replacement of all simulated stubs with live, functional protocol and hardware bridge integrations.

## Goals / Non-Goals

**Goals:**
- Replace the mock `FibreAdapter` with an authentic binary Fibre RPC client in JavaScript:
  - Byte framing: `0xAA` sync byte, packet length, CRC8 header, CRC16 payload.
  - Endpoint 0 JSON schema request and schema tree decoding.
  - Typed binary reading and writing of motor target, velocity estimate, angle, and Iq current.
- Implement live CAN Bus transport in `webui/barefoc.html`:
  - Connect to AgentLA's active WebSocket bridge `/api/v1/can/ws`.
  - Encode and decode SimpleFOC CAN frames and ODrive CAN messages (command IDs, input pos, input vel, heartbeat).
- Replace the mock `MultiBusAdapter` with live AgentLA I2C/SPI bridge calls:
  - Dispatch register reads and writes to physical target control tables over AgentLA's hardware bus.
- Replace the artificial 400ms JS timer in the Alignment Wizard:
  - Sequence real SimpleFOC alignment commands (`M1` alignment routines) asynchronously over the active transport.
  - Await target hardware confirmation, read back `zero_electric_angle` and direction, and update calibration parameters.
- Wire gate driver configuration:
  - Allow reading and writing SPI registers for TI DRV8316 and DRV8311.
- Maintain strict `#![no_std]` compliance and zero emojis.

**Non-Goals:**
- Modifying AgentLA core firmware binaries; all bridges (`/api/v1/can/ws`, `/api/v1/terminal/ws`, `/api/v1/bridge/i2c`) already exist in firmware.

## Decisions

### D1: Native Binary Fibre Client in Pure JS
- **Decision**: Implement a compact, zero-dependency `FibreClient` class in `webui/barefoc.html` handling packet framing, CRC8/CRC16 calculation, and endpoint lookup.
- **Rationale**: Enables direct communication with native ODrive boards and `barefoc-sim` running on host port 8000 without requiring Node.js or Python daemons.

### D2: Unified CAN Message Multiplexing
- **Decision**: Connect to `/api/v1/can/ws` and multiplex both SimpleFOC CAN IDs (standard 11-bit) and ODrive CAN frame arbitration IDs (6-bit node ID + 5-bit command ID).
- **Rationale**: Allows the user to switch seamlessly between SimpleFOC and ODrive CAN motor nodes on the same physical bus.

### D3: Asynchronous Hardware Alignment State Machine
- **Decision**: Replace `setInterval` with an asynchronous Promise-based state machine that sends alignment commands, checks execution progress, and parses the returned `zero_electric_angle` float.
- **Rationale**: Ensures the GUI accurately reflects the target motor's physical calibration state rather than guessing completion on a timer.

## Risks / Trade-offs

- **[Risk] High CAN bus traffic flooding the browser WebSocket**
  - **Mitigation**: Filter messages client-side by Target Node ID, only parsing telemetry frames matching the configured motor node.
- **[Risk] Fibre schema payload exceeding browser memory during initial introspection**
  - **Mitigation**: Parse and retain only motor/axis objects from the Endpoint 0 schema, discarding unneeded subtrees.
