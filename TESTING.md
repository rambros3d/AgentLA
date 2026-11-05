# AgentLA Hardware Testing Setup & Verification Guide

This document describes the multi-target hardware testing setup, physical pin mappings, autonomous reflashing loop, and verification procedures for the **AgentLA RP2350 Debug & Programming Probe** running Embassy Rust firmware (`firmware-rs`, branch `embassy`).

---

## 1. System Overview

```
                          +-------------------------------------------------+
                          |           Host Workstation (Linux)              |
                          +-------------------------------------------------+
                                      |                       |
                             /dev/ttyACM1            /dev/ttyACM0
                      (AgentLA Target Telemetry) (Pico CMSIS-DAP Probe)
                                      |
                 +--------------------+---------------------+
                 |       AgentLA RP2350 Probe (Pico 2)       |
                 |      ARM Cortex-M33 @ 150MHz (Rust)      |
                 +--------------------+---------------------+
                         |            |             |
                   Port 1|      Port 2|       Port 3|
                   (SWD) |    (RVSWD) |      (SWIO) |
                         v            v             v
                 +------------+ +------------+ +------------+
                 |  STM32F103 | |  CH32V203  | |  CH32V003  |
                 |  BluePill  | |   WeAct    | |    WCH     |
                 | (Cortex-M3)| | (QingKe V4)| | (QingKe V2)|
                 +------------+ +------------+ +------------+
```

- **Probe MCU**: Raspberry Pi Pico 2 (Raspberry Pi RP2350A, Dual ARM Cortex-M33 @ 150 MHz).
- **Firmware**: Embassy asynchronous Rust framework (`firmware-rs`).
- **USB Device Identity**: `1209:414d Generic Adi Linden`
  - `/dev/ttyACM1`: Single CDC-ACM port dedicated to transparent Target Telemetry / UART Bridge (GPIO 14 RX / GPIO 8 TX). (Legacy CDC-ACM ASCII text CLI retired).
  - CMSIS-DAP v1 (HID) & CMSIS-DAP v2 (Bulk) high-speed hardware debugger interfaces.
  - CDC-NCM Ethernet: `10.10.10.10/24` for high-throughput HTTP/REST API, JSON-RPC MCP server, and WebSockets.
- **Auxiliary Probe**: Raspberry Pi Pico running official Raspberry Pi Debugprobe CMSIS-DAP firmware (`2e8a:000c`) on `/dev/ttyACM0` for optional external SWD hardware tracing.

---

## 2. Dedicated Multi-Target Pinout & Wiring Matrix

Each target MCU connects to dedicated physical GPIO pins on the AgentLA probe, allowing simultaneous attachment without jumper switching:

| Target Port | Target MCU | Architecture & Protocol | Probe Pin | Target Pin | Target Signal Description |
|---|---|---|---|---|---|
| **Port 1** | **STM32F103C8T6** (BluePill) | ARM Cortex-M3 (ADIv5 SWD) | **GPIO 4** | **SWCLK** (PA14) | SWD Serial Wire Clock |
| | | | **GPIO 5** | **SWDIO** (PA13) | SWD Serial Wire Data Bidirectional |
| | | | **GPIO 6** | **NRST** | Active-Low Hardware Reset |
| | | | **3.3V** | **3.3V** | Target Power (from common rail) |
| | | | **GND** | **GND** | Common Ground |
| **Port 2** | **WeAct Studio CH32V203C8T6** | QingKe V4 RISC-V (2-Wire RVSWD) | **GPIO 8** | **SWCLK** (PA14) | RVSWD Clock |
| | | | **GPIO 9** | **SWDIO** (PA13) | RVSWD Data Bidirectional |
| | | | **GPIO 10** | **NRST** | Active-Low Hardware Reset |
| | | | **3.3V** | **3.3V** | Target Power (from common rail) |
| | | | **GND** | **GND** | Common Ground |
| **Port 3** | **WCH CH32V003F4P6** | QingKe V2 RISC-V (1-Wire SWIO) | **GPIO 12** | **SWIO** (PD1, Pin 8) | Single-Wire Half-Duplex Debug IO |
| | | | **GPIO 13** | **NRST** (PD7, Pin 7) | Active-Low Hardware Reset |
| | | | **GPIO 14** | **TX** (PD4 / PC4) | Telemetry UART Serial Input |
| | | | **3.3V** | **3.3V** | Target Power (from common rail) |
| | | | **GND** | **GND** | Common Ground |

---

## 3. Autonomous Hands-Free Reflashing Loop

Zero physical button touches (`BOOTSEL` button) are required to build, flash, and test new firmware iterations on the AgentLA probe:

1. **Software Trigger**: HTTP `POST http://10.10.10.10/api/v1/system/bootsel` calls `embassy_rp::rom_data::reset_to_usb_boot(0, 0)`.
2. **Bootloader Re-enumeration**: RP2350 reboots directly into USB mass-storage bootloader mode (`2e8a:000f Raspberry Pi RP2350 Boot`).
3. **Flashing**: `picotool load -x /tmp/firmware-rs.uf2` uploads the binary and reboots into user application mode.
4. **CDC-ACM Re-enumeration**: The probe appears back on `/dev/ttyACM1` (dedicated to transparent Target UART telemetry) within ~1.5 seconds.

### Automated Reflash Script

```bash
cd firmware-rs

# 1. Compile release binary
cargo build --release

# 2. Convert ELF to RP2350 UF2 format
cp target/thumbv8m.main-none-eabihf/release/firmware-rs /tmp/firmware-rs.elf
picotool uf2 convert --family rp2350-arm-s /tmp/firmware-rs.elf /tmp/firmware-rs.uf2

# 3. Trigger reboot into BOOTSEL via REST API and load firmware
curl -s -X POST http://10.10.10.10/api/v1/system/bootsel || true
sleep 2
picotool load -x /tmp/firmware-rs.uf2
```

---

## 4. Target Programming, Diagnostics & MCP Surface

The legacy ASCII text CLI previously running on CDC-ACM0 has been retired because all probe management, diagnostics, bus bridges, and debugger functions are fully available via the CDC-NCM REST API (at `http://10.10.10.10/`) and the JSON-RPC MCP server.

### REST API Reference

| Endpoint | Method | Description |
|---|---|---|
| `/api/board/manifest` | GET | Retrieve board manifest and identity |
| `/api/v1/system` | GET | Probe system telemetry, uptime, clocks |
| `/api/v1/system/bootsel` | POST | Reboot probe into USB BOOTSEL flash mode |
| `/api/v1/programmer/detect` | GET / POST | Scan and detect targets on Port 1 (SWD), Port 2 (RVSWD), and Port 3 (SWIO) |
| `/api/v1/programmer/halt` | POST | Halt target core execution (`bluepill`, `ch32v203`, `ch32v003`) |
| `/api/v1/programmer/resume` | POST | Resume target core execution (`bluepill`, `ch32v203`, `ch32v003`) |
| `/api/v1/programmer/flash/read` | POST | Read target flash memory page / vector table |
| `/api/v1/programmer/flash/erase` | POST | Erase target flash page or sector |
| `/api/v1/programmer/flash/write` | POST | Write target flash memory |
| `/api/v1/bus/uart/write` | POST | Write raw bytes to physical target UART |
| `/api/v1/bus/uart/read` | GET | Read received bytes from target UART buffer |
| `/api/v1/bridge` | GET | Bus bridge and peripheral status |
| `/mcp/health` | GET | MCP server health check |
| `/mcp` | POST | MCP JSON-RPC protocol endpoint (14 AI agent tools) |

### Transparent Target Telemetry (CDC-ACM)

The single USB CDC-ACM port (`/dev/ttyACM1` or first available ACM port) is directly dedicated to transparent bi-directional target UART telemetry (GPIO 14 RX / GPIO 8 TX @ 115,200 baud). Host serial terminals (e.g. `picocom -b 115200 /dev/ttyACM1` or python serial scripts) communicate directly with target application logs and console IO.

---

## 5. Live Test Results

### 1. `status` & Pin Voltage Telemetry
```
agentla> status
SYSTEM: OK
CLOCKS: 150 MHz sys_clk
NET: CDC-NCM 10.10.10.10/24 up
PROGRAMMER: Ready (SWD/SWIO/RVSWD multi-port)
GPIO IN: 0x000037F0
  P1: CLK4=1 DIO5=1 RST6=1
  P2: CLK8=1 DIO9=1 RST10=1
  P3: SWIO12=1 RST13=1 TX14=0
```

### 2. Multi-Target Detection (`target detect`)
```
agentla> target detect
Scanning all target ports...
[Port 1: BluePill (GPIO 4 SWCLK, 5 SWDIO, 6 nRST)]
  Name:     STM32F103 Medium Density
  Family:   STM32F1
  Flash:    128 KB
  RAM:      20 KB
[Port 2: CH32V203 (GPIO 8 SWCLK, 9 SWDIO, 10 nRST)]
  Name:     WCH CH32V203 / CH592 (QingKe V4 RISC-V 2-Wire)
  Family:   CH32V203 (2-Wire)
  Flash:    64 KB
  RAM:      20 KB
[Port 3: CH32V003 (GPIO 12 SWIO, 13 nRST, 14 TX)]
  No target detected on Port 3.
```

### 3. Flash Vector Table Read (`flash read`)
```
agentla> flash read
Port 1 [BluePill] Flash [0x08000000..0x08000010]:
  0x08000000 (SP):           0x20005000
  0x08000004 (Reset Vector): 0x08001EAD
  0x08000008 (NMI Vector):   0x08001EF5
  0x0800000C (HardFault):    0x08001EF5
  Raw bytes: 00 50 00 20 AD 1E 00 08 F5 1E 00 08 F5 1E 00 08 
```
- **Main Stack Pointer (`0x20005000`)**: Exactly matches the top of STM32F103 20 KB SRAM (`0x20000000` + `20 * 1024`).
- **Reset Vector (`0x08001EAD`)**: Points to active Thumb-mode firmware entry point in Flash.

### 4. Core Halt & Resume Control
```
agentla> target halt
Port 1 [BluePill] core halted successfully.

agentla> target resume
Port 1 [BluePill] core resumed.

agentla> target halt ch32v203
Port 2 [CH32V203] core halted successfully.

agentla> target resume ch32v203
Port 2 [CH32V203] core resumed.
```

---

## 6. Key Hardware Architecture Discoveries & Protocol Insights

1. **RP2350 Pad Isolation Control (`ISO` bit)**:
   - On the RP2350 (unlike RP2040), `PADS_BANK0.gpio(pin)` includes bit 8 (`ISO`), which defaults to `1` (isolated) at reset.
   - When `ISO = 1`, the physical pad is disconnected from the chip's internal logic: inputs read `0` and outputs fail to drive. Custom pad configurations **must** explicitly call `w.set_iso(false)`.

2. **PIO Output Directions (`PINDIRS`)**:
   - `swd_probe.pio` and `rvswd.pio` dynamically manage bidirectional data pins (`SWDIO`) via `out pindirs, 1`. Clock pins (`SWCLK`) are driven by side-set and must have their initial direction explicitly configured as outputs via `pio_set_pindirs(sm, clk_pin, 1, true)`.

3. **Open-Drain 1-Wire PIO Setup**:
   - When using side-set pin direction (`side_pindir = true`) for open-drain protocols (WCH 1-wire SWIO), the state machine's output register for the pin must be initialized to logic `0` (`set pins, 0`). When the pin direction is driven as output (`side 1`), it pulls low to 0V; when input (`side 0`), it floats high via pull-up resistor.

4. **ARM ADIv5 SWD Initialization Sequencing**:
   - After line reset / `JTAG-to-SWD` switching sequence, the host **must** first read `DP_DPIDR` (register 0x00).
   - Before attempting any Access Port (AHB-AP) transfers, the debug power domain must be powered up by setting `CDBGPWRUPREQ` and `CSYSPWRUPREQ` in `DP_CTRLSTAT` (`0x50000000`) and verifying the handshake response (`0xA0000000`).
   - Memory accesses into Flash incur bus wait states; the transport retry loop must handle `SWD_ACK_WAIT` (0x02) during both `AP_DRW` read triggers and `DP_RDBUFF` collection.
5. **Timing errata (observed 2026-11-05)**:
   - After the `bootsel` serial command, the RP2350 can take **longer than 2 s** to re-enumerate as `2e8a:000f`; wait **3–5 s** before invoking `picotool`. A premature `picotool load/reboot` exits with status 249 ("No accessible RP-series devices") and leaves the board in BOOTSEL — just rerun the command once enumeration completes.
   - `curl -T file URL` issues a **PUT**, not a POST. The tarball installer accepts only POST: use `curl -X POST --data-binary @app.tar http://10.10.10.10/api/v1/apps/install/{name}`.

---

## 7. Phase 1 Hardware Verification (LittleFS / App Ecosystem, 2026-11-05)

Firmware built from `firmware-rs` (branch `embassy`), flashed via the §3 hands-free
loop (bootsel + `picotool load -x`, 480 KB UF2, ~8 MB debug ELF). All checks below
were run against `http://10.10.10.10` over the CDC-NCM virtual Ethernet link.

| # | Check | Result |
|---|---|---|
| 1 | Build release | OK — no new warnings (only pre-existing `proc-macro-error2` future-incompat note) |
| 2 | Defmt mount line | Mount at `0x00190000`; on first ever boot the format fallback ran once, subsequent boots mount directly |
| 3 | Embedded fallback `/` | OK — INDEX_HTML served unchanged |
| 4 | `GET /api/v1/apps` | OK — JSON array, seeded `servo-master` present |
| 5 | `POST /api/v1/apps/install/{name}` | OK — `{"status":"installed","app":"live-test","files":2,"bytes":75}` |
| 6 | `GET /apps/{name}/index.html` | OK — HTTP 200, `Content-Type: text/html`, chunked streaming from LittleFS |
| 7 | `GET /api/v1/apps/storage` | OK — `total_bytes:2097152, used:32768→40960→32768, block_count:512` |
| 8 | `DELETE /api/v1/apps/{name}` | OK — `{"status":"deleted"}`, subsequent `GET /apps/…` → `404 {"status":"not_found"}` |
| 9 | SSE `/api/v1/events` + `/api/v1/system` | OK — telemetry events flowing, endpoints unaffected by FS traffic |
| 10 | Reboot persistence | OK — installed app survived flashing **and** `picotool reboot` cycles (LittleFS mount without re-format) |

Operational notes:
- `used_bytes` rose 32768 → 40960 (2 × 4 KiB blocks) on install and fell back exactly
  on delete — free-space accounting is consistent, and space freed by `DELETE` is
  reclaimed at the filesystem level.
- The app-list JSON splices an installed manifest only when it is bounded printable
  ASCII JSON (`{`…`}` with no trailing newline); a raw manifest is always available via
  `GET /api/v1/apps/{name}/manifest`.
- A brief connection-refused blip was observed on the HTTP port immediately after a
  tarball install while flash writes settle; the next request succeeded. Not a blocker,
  but worth re-checking when the connection pool / keep-alive behavior is tuned.
- UNTouched: profile slots below `0x190000`, all programmer/analyzer subsystems.
