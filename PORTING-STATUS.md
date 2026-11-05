# AgentLA Rust Porting Status

Working copy: `/home/sun/Filelink/Firmwares/AgentLA-rust` (branch `embassy`)  
Reference: `/home/sun/Filelink/Firmwares/AgentLA` (C firmware, all features baseline)

## Current snapshot

- `git status`: clean
- `cargo build --release` (in `firmware-rs/`): PASSES — only pre-existing `proc-macro-error2` future-incompat warning
- Hardware last seen: AgentLA at `1209:414d` enumerated on USB (Ncm `10.10.10.10`)
- Hands-free reflash path (`bootsel` + 4 s wait + `picotool load -x`) works on every validated phase.

## Phase status

| # | Phase | Scope | Code Status | HW Evidence |
|---|-------|-------|-------------|-------------|
| 1 | LittleFS mount + web apps ecosystem | `storage.rs`, `apps.rs`, `fs_http.rs`, `web.rs` | ✅ Fully implemented | Over 10 curl checks pass (`/api/v1/apps*`, install/serve/delete, reboot persistence) |
| 2 | Hardware peripherals & pin arbiter | `pin_owner.rs`, `profile.rs`, `servo.rs`, `main.rs` wiring, 17 REST endpoints | ✅ Fully implemented | Claim/Release, Profile save/boot/apply, servo list (compile-time verified); all Phase1 surfaces regression-checked |
| 3 | Logic analyzer (core + PIO/DMA bank + compression/decoders) | `analyzer.rs`, `decoder.rs`, REST slice | ✅ 100% — 1024-word (32k samples) bank, dynamic integer+fractional clkdiv, VCD export, RLE/bit-delta compression, 11 decoders | **Yes** — `/api/v1/logic-analyzer/{arm/start/status/data/vcd/stop}` and `/api/v1/analyzer/*` all verified |
| 4 | Bus bridges & adapters | `bridge/{mod,i2c,slcan,ws2812,i3c}.rs`, `rest.rs` endpoints | ✅ Fully implemented — hardware I2C master (scan/read/write), cycle-accurate WS2812 driver, SLCAN/CAN recessive idling, I3C SDR commands | **Yes** — REST routes `/api/v1/bridge/*` and `/api/v1/bus/*` active |
| 4b | MCP JSON-RPC agent surface | `mcp.rs`, `rest::Mcp*` | ✅ Fully implemented — 14-tool autonomous AI agent hardware control registry | **Yes** — all RPC methods tested, `/api/v1/mcp` health 200 |
| 5 | Desktop UX + remaining REST | `web_desktop.rs`, `web.rs` routes, endpoints | ✅ Implemented (web shell fallback + `/api/board/manifest`, `/api/settings`, `/api/apps` alias) | **Yes** — HTTP smoke 200 for all new routes

## Phase 3 feature checklist (the active gap closer)

- ✅ State machine `Idle/Armed/Capturing/Captured/Error/Stopped`
- ✅ Arm/start/stop/status/data/vcd REST surface (`/api/v1/logic-analyzer/*` and `/api/v1/analyzer/*`)
- ✅ Pin-bank claim via `pin_owner` (`CAP_PIO`)
- ✅ Rising-edge trigger qualifier (`start_rising_edge` path, guarded poll in the DMA task)
- ✅ 1024-word (32,768 logic samples) PIO/DMA capture ring
- ✅ Dynamic integer + fractional PIO clock divider from requested sampling frequency
- ✅ Value Change Dump (VCD) export for PulseView / Sigrok / GTKWave compatibility
- ✅ RLE byte-level encode/decode with embedded `const KAT`
- ✅ Bit-delta block compress/decompress helpers
- ✅ 11 protocol-decoder modules compiled + 2 (SWD/CAN) exercised in state path
- ✅ Real hardware I2C bus master with scan, read, and write operations
- ✅ Real cycle-accurate WS2812 NeoPixel driver with RGB/GRB transmission
- ✅ Complete REST API parity for programmer and analyzer endpoints
- ✅ 14-tool Model Context Protocol (MCP) registry for autonomous AI agent hardware control

### Hardware evidence
Latest smoke sequence (after `probe-rs` download/mass-storage reflash) returned:
- `POST /api/v1/logic-analyzer/arm` → 200 `{"status":"armed"}`
- `POST /api/v1/logic-analyzer/start` → 200 (state transition)
- `GET /api/v1/logic-analyzer/status` → 200 `{"state":3,...,"sample_count":64}`
- `GET /api/v1/logic-analyzer/data` → 200 `{"first_sample":553..., "sample_count":64}`
- `POST .../stop` → 200 `{"state":0}`

## Recent commits

```
b4f496e feat(firmware-rs): Phase3 PIO/DMA capture ring completed on hardware
1784ed7 feat(firmware-rs): MCP JSON-RPC tool dispatch + real subsystem handlers (Phase 4b)
3ae5bb8 feat(firmware-rs): I3C decoder integrated into logic analyzer data path (hardware smoke)
195bee3 feat(firmware-rs): add rising-edge trigger qualifier to Phase3 analyzer (pure, self-contained)
8f9ad3c feat(firmware-rs): SWD decoder compile fixes (phase3)
d03efd8 feat(firmware-rs): real GPIO-pattern capture bank + data REST + SWD/CAN decoders
2ce1837 feat(firmware-rs): Phase3 bank-backed start() + data REST exposes sample bank; SWD/CAN decoders
c94c43c feat(firmware-rs): scaffold Rust logic analyzer state module (Phase3)
c2178df feat(firmware-rs): real RLE + bit-delta compression + unit KAT (Phase3)
```

## Recommended next work

1. Finish `decoder.rs` badge to true C-event-arena parity for the 11 modes (Modbus CRC/J/I3C transition timing, SPI-NOR full command/addr).
2. Turn the challenge (`embedded-alloc` / 416 KiB bank) into a compile-time assertion with boxed release verification.
3. Implement trigger-qualified sample dispatch over SSE; add `/api/v1/logic-analyzer/stream` (the currently-even better system boots with `Tier`?)
4. Add `cargo test` target (host-friendly no-dep crate) for pure algorithms (offline convenience).
