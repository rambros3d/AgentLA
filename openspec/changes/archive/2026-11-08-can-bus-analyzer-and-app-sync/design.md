## Context

AgentLA incorporates hardware CAN transceiver support (TCAN334 on GPIO 18/19 with software-switchable $120\,\Omega$ termination on GPIO 22). While USB SocketCAN (`gs_usb`) and SLCAN interfaces exist, users interacting with AgentLA via its browser desktop workbench need an integrated in-browser CAN analyzer tool. 

Currently, `webui/apps.html` advertises a "CAN Bus Analyzer" (`can-sniffer`), but the directory `webui/apps/can-sniffer` is absent. Furthermore, `apps/` and `webui/apps/` are diverging, and `firmware-rs/src/apps.rs` embeds static dummy HTML strings in ROM flash for seeding.

This design establishes:
1. A lean, decoupled app seeding and serving strategy without static HTML strings in firmware ROM.
2. An asynchronous, full-duplex WebSocket CAN bridge (`/api/v1/can/ws`) in `firmware-rs`.
3. A rich, responsive CAN Bus Analyzer web application (`/apps/can-sniffer/`) matching the AgentLA desktop design language.

## Goals / Non-Goals

**Goals:**
- Provide a dedicated CAN Bus Analyzer web app in `webui/apps/can-sniffer/` and `apps/can-sniffer/` with real-time frame sniffer table, ID filter, frame transmission, and termination control.
- Implement `/api/v1/can/ws` on the Embassy async executor in `firmware-rs` for low-latency bidirectional CAN frame streaming.
- Strip hardcoded HTML string constants from `firmware-rs/src/apps.rs`, freeing flash space in firmware slots.
- Synchronize root `apps/` with `webui/apps/` and ensure `tools/pack_webui.py` bundles `can-sniffer` into `webui.img`/`webui.uf2`.

**Non-Goals:**
- Heavy client-side DBC database compiling (only lightweight common decoders like OBD-II standard PIDs in the browser).
- Replacing kernel SocketCAN `gs_usb` (the WebSocket interface complements USB and REST).
- Re-architecting the LittleFS driver.

## Decisions

### Decision 1: JSON Framing over WebSocket (`/api/v1/can/ws`)
- **Rationale**: Serial Terminal uses WebSocket binary/text streaming at `/api/v1/terminal/ws`. For CAN frames, JSON objects `{ "id": 0x123, "ext": false, "rtr": false, "dlc": 8, "data": "0102...", "ts": 123456 }` offer immediate, robust browser deserialization without bespoke binary packing, while remaining lightweight enough for RP2350 microcontrollers at standard CAN bus loads (up to 1 Mbps).
- **Alternative Considered**: Lawicel SLCAN ASCII text stream. While compact, SLCAN requires custom regex string parsing in the browser and does not natively carry microsecond timestamps.

### Decision 2: Zero HTML Strings in Firmware ROM Flash
- **Rationale**: Firmware slots (Slot A at `0x10004000`, Slot B at `0x10130000`) have fixed partitions. Storing large HTML web apps in ROM constants wastes firmware capacity and creates maintenance drift. Seeding in `firmware-rs/src/apps.rs` will only create directories and minimal manifest files (`app.json`), while the actual HTML/JS/CSS assets reside in the `SYSTEM_FS` partition (packed via `tools/pack_webui.py`).
- **Alternative Considered**: Embedding compressed gzip strings in ROM. Still consumes firmware slot budget unnecessarily when a dedicated 4.5 MB LittleFS WebUI partition exists.

### Decision 3: Physical Layer Controls via Existing REST + WebSocket
- **Rationale**: Bus speed configuration and $120\,\Omega$ termination control are routed through `/api/v1/bridge/can/termination` and `/api/v1/bridge/can`, while live traffic flows through `/api/v1/can/ws`. This maintains clean separation between physical pin arbitration and real-time packet streaming.

## Risks / Trade-offs

- **[Risk] High-rate CAN frame bursts exceeding WebSocket buffer capacity** → **Mitigation**: The firmware CAN queue will use ring-buffer dropping with an overflow counter flag in telemetry, and the web UI will throttle table DOM insertion using `requestAnimationFrame`.
- **[Risk] Missing WebUI partition on unformatted hardware** → **Mitigation**: When `SYSTEM_FS` is unmounted or empty, launcher displays clear guidance to flash `webui.uf2`, and manifests still report app identity correctly.
- **[Risk] Pin conflicts when CAN is claimed** → **Mitigation**: Respect `agentla-pin-arbiter` / `pin_owner` contract. CAN bridge claims GPIO 18, 19, and 22 under `Driver::Adapter`.
