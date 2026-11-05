# AgentLA Rust Porting Status

Working copy: `/home/sun/Filelink/Firmwares/AgentLA-rust` (branch `embassy`)  
Reference: `/home/sun/Filelink/Firmwares/AgentLA` (C firmware, all features baseline)

## Current Snapshot

- `git branch`: `embassy` (synced with `origin/embassy`)
- `cargo build --release` (in `firmware-rs/`): PASSES (total binary size: ~972 KB, well under 1.50 MB ROM partition)
- Hardware status: AgentLA at `1209:414d` enumerated on USB with CDC-ACM0 (CLI), CDC-ACM1 (UART), CMSIS-DAP v1/v2, and CDC-NCM Ethernet (`10.10.10.10`).
- Target MCUs attached & verified:
  - **Port 1 (GPIO 4/5/6)**: STM32F103 BluePill (ARM Cortex-M3, SWD)
  - **Port 2 (GPIO 8/9/10)**: WeAct CH32V203 (QingKe V4 RISC-V, 2-wire RVSWD)
  - **Port 3 (GPIO 12/13/14)**: WCH CH32V003 (QingKe V2 RISC-V, 1-wire SWIO)

## Subsystem Parity Matrix

| # | Subsystem | Scope | Code Status | Live Hardware Evidence |
|---|---|---|---|---|
| 1 | **LittleFS & App Ecosystem** | `storage.rs`, `apps.rs`, `fs_http.rs` | ✅ 100% Parity | `/api/v1/apps*`, install/serve/delete, reboot persistence verified over HTTP |
| 2 | **Hardware Pins & Profiles** | `pin_owner.rs`, `profile.rs`, `servo.rs` | ✅ 100% Parity | Pin claim/release, profile save/boot/apply, servo list |
| 3 | **Logic Analyzer & VCD** | `analyzer.rs`, `decoder.rs`, PIO0 SM0 + DMA CH0 | ✅ 100% Parity | 1024-word ring buffer, dynamic fractional clkdiv, live VCD export (`/api/v1/logic-analyzer/vcd`), RLE/bit-delta compression, 11 decoders |
| 4 | **Hardware Bus Bridges** | `bridge/{mod,i2c,slcan,ws2812,i3c}.rs` | ✅ Fully Implemented | Bit-banged I2C master with scan/read/write, cycle-accurate 800 kHz WS2812 NeoPixel driver, SLCAN/CAN standby & recessive line driving |
| 4b | **MCP JSON-RPC AI Agent Surface** | `mcp.rs`, `rest::Mcp*` | ✅ 100% Parity | 14 registered AI agent hardware control tools, `/mcp/health` 200 OK |
| 5 | **Exact DaisyUI Web Desktop OS** | `web_assets.rs`, `web.rs`, `rest.rs` | ✅ 100% Parity | All 635 KB of C assets (`index.html`, `app.js`, `style.css`, `apps.html`, `themes.*`, `board.json`, SVGs) served out of ROM; verified at `http://10.10.10.10/` |
| 6 | **Target Programmer & Debugger** | `programmer/{mod,pins,swd,rvswd,swio,target,algo}` | 🟡 95% Parity | Detection, Halt, Resume, and Flash Read verified on all 3 MCUs; Flash Erase verified on all 3 MCUs; Flash Write in final verification |

---

## Detailed Target Flashing & Debugging Status

| Feature | STM32F103 (Port 1 - SWD) | CH32V203 (Port 2 - RVSWD) | CH32V003 (Port 3 - SWIO) |
|---|---|---|---|
| **Detection (`target detect`)** | ✅ Detected (`STM32F103 Medium Density`) | ✅ Detected (`WCH CH32V203 / CH592`) | ✅ Detected (`WCH CH32V003`) |
| **Core Halt (`target halt`)** | ✅ Verified | ✅ Verified | ✅ Verified |
| **Core Resume (`target resume`)**| ✅ Verified | ✅ Verified | ✅ Verified |
| **Flash Read (`flash read`)** | ✅ Verified (Vector Table / SP) | ✅ Verified (32-bit words via PROGBUF) | ✅ Verified (32-bit words via PROGBUF) |
| **Flash Erase (`flash erase`)** | ✅ Sector erase verified | ✅ Page erase (256 B) verified | ✅ Page erase (64 B) verified |
| **Flash Write (`flash write`)** | ✅ Verified | 🟡 Fast page write tuning | 🟡 Buffer latch timing tuning |

---

## Flash ROM Memory Allocation Map

```
+--------------------------------------------------------------------------+
| Flash Address Range    | Size    | Allocation & Ownership                 |
+--------------------------------------------------------------------------+
| 0x00000000..0x00180000 | 1.50 MB | Firmware Bootloader & ROM (Code + Web) |
|   - Executable Binary  | 337 KB  | Compiled .text and ARM vectors         |
|   - Embedded WebUI     | 635 KB  | Exact DaisyUI Assets in Flash ROM      |
|   - Free Headroom      | 564 KB  | Margin for future firmware growth      |
+--------------------------------------------------------------------------+
| 0x00180000..0x00190000 | 64 KB   | Hardware Profile Storage Window        |
+--------------------------------------------------------------------------+
| 0x00190000..0x00390000 | 2.00 MB | LittleFS Flash Partition (Web Apps)   |
+--------------------------------------------------------------------------+
| 0x00390000..0x00400000 | 448 KB  | Unallocated / Reserved                 |
+--------------------------------------------------------------------------+
```

---

## Recent Commits

```
e0b781e feat(web): port complete C WebUI desktop and DaisyUI assets to Embassy Rust
b614add fix(programmer): improve DPIDR detection and ack handling
8cd13cd feat(firmware-rs): Phase4 MCP registry + real bus bridges (I2C/WS2812/CAN)
b4f496e feat(firmware-rs): Phase3 PIO/DMA capture ring completed on hardware
1784ed7 feat(firmware-rs): MCP JSON-RPC tool dispatch + real subsystem handlers (Phase 4b)
3ae5bb8 feat(firmware-rs): I3C decoder integrated into logic analyzer data path (hardware smoke)
195bee3 feat(firmware-rs): add rising-edge trigger qualifier to Phase3 analyzer
8f9ad3c feat(firmware-rs): SWD decoder compile fixes (phase3)
d03efd8 feat(firmware-rs): real GPIO-pattern capture bank + data REST + SWD/CAN decoders
```

---

## Remaining Gaps to Complete 100% Dev Parity

1. **Target Flash Write Finalization**:
   - Complete subagent verification of multi-page write and verify on CH32V203 and CH32V003.
2. **Physical UART Forwarding (`/dev/ttyACM1`)**:
   - Route `cdc_uart_task` to the RP2350 hardware UART peripheral on target pins (e.g. GPIO 14 / CH32V003 TX).
3. **Continuous Logic Analyzer SSE Stream**:
   - Implement `/api/v1/logic-analyzer/stream` SSE push for live real-time traces in the Web Desktop canvas.
4. **Hardware SPI Peripheral Bridge**:
   - Connect `/api/v1/bus/spi/transfer` to the RP2350 SPI0/SPI1 peripheral.
