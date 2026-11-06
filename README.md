# AgentLA

**AgentLA** is an open-source, all-in-one embedded hardware bring-up and instrumentation lab in a single USB probe. Built from the ground up in memory-safe **Embassy async Rust** for the Raspberry Pi RP2350 (and RP2040), AgentLA combines a multi-target autonomous programmer, standard CMSIS-DAP debugger, high-speed logic analyzer, 4-channel digital oscilloscope, arbitrary signal generator, glitch injector, bus bridges, and host-native Linux kernel USB interfaces.

Plug it in over USB, open a browser at `http://agentla.local` (or `http://10.10.10.10`), or connect your favorite AI coding agent via the native **Model Context Protocol (MCP)** server — no proprietary drivers, host toolchains, or vendor software required.

---

## Architecture & Hardware Highlights

* **100% Embassy Async Rust**: Built on the modern embedded Rust ecosystem (`embassy-rp`, `embassy-net`, `embassy-usb`) for deterministic latency, zero race conditions, and rock-solid stability.
* **Modular BSP Architecture**: Decoupled Cargo workspace structure with dedicated board crates (`boards/pico2`, `boards/bakuprobe`, and `boards/templates` for custom PCBs). Reusable drivers for external ICs (TI PGA113 amplifier, TI INA180 current sense, TCAN334 CAN-FD transceiver).
* **Host-Native Kernel USB Interfaces**:
  * **SocketCAN (`gs_usb`)**: Native Linux network device (`can0`) with hardware CAN-FD transceiver control and switchable $120\,\Omega$ termination.
  * **Linux I2C (`i2c-tiny-usb`)**: Native Linux I2C bus adapter (`/dev/i2c-X`) compatible with standard `i2c-tools` (`i2cdetect`, `i2cget`, `i2cset`).
  * **CMSIS-DAP v1 & v2**: Zero-driver USB HID (v1) and high-speed WinUSB bulk endpoints (v2) compatible with **probe-rs**, **PyOCD**, **OpenOCD**, and Keil MDK.
  * **Transparent UART Telemetry**: Dedicated single CDC-ACM serial port (`/dev/ttyACM1`) directly connected to target UART (GPIO 14 RX / GPIO 8 TX) for streaming debugging.
  * **Virtual Ethernet (`CDC-NCM`)**: High-throughput network interface serving the Web Desktop UI, REST API, and MCP server at `10.10.10.10`.
* **Zero Host Internet Interruption**: Link-local DHCP configuration assigns host IP without overriding default gateways or DNS resolvers.

---

## Hardware Pin Allocation & Zones

AgentLA features clean functional pin zones designed for maximum usability and signal integrity:

| Functional Zone | Default Pin Range | Description & Target Interfaces |
| :--- | :--- | :--- |
| **PROGRAMMER** | `GPIO 4 – 14` | Fixed multi-port programmer headers:<br>• **Port 1 (GPIO 4 SWCLK, 5 SWDIO, 6 nRST)**: ARM Cortex-M SWD (STM32, Puya, nRF)<br>• **Port 2 (GPIO 8 CLK, 9 DIO, 10 nRST)**: WCH 2-Wire RVSWD (CH32V203 / V307 / CH592)<br>• **Port 3 (GPIO 12 SWIO, 13 CLK, 14 TX/nRST)**: WCH 1-Wire SWIO (CH32V003) |
| **BUS ADAPTERS** | `GPIO 16 – 22` | Host-native bridges & bus masters:<br>• **CAN**: GPIO 18 (TX), GPIO 19 (RX), GPIO 22 ($120\,\Omega$ Term)<br>• **I2C**: GPIO 16 (SDA), GPIO 17 (SCL)<br>• **SPI**: GPIO 16–19 (MISO, CS, SCK, MOSI)<br>• **UART Telemetry**: GPIO 8 (TX), GPIO 14 (RX) |
| **FLEXIBLE INSTRUMENTS** | `GPIO 20 – 29` | Dynamic pin pool for Logic Analyzer & Mixed-Signal Instruments:<br>• **Oscilloscope (MSO)**: GPIO 26–29 (SAR ADC Ch 0–3, 500 kS/s)<br>• **Signal Generator**: PWM slices 0–7 (GPIO 10–29, 1 Hz to 75 MHz)<br>• **Logic Analyzer**: 10 simultaneous channels with hardware trigger engine |

---

## Core Capabilities

### 1. Multi-Target Standalone Programmer
Autonomous on-probe flashing engine capable of detecting, reading, erasing, programming, and verifying target microcontrollers directly over REST, Web UI, or MCP:

* **Supported Architectures**:
  * **ARM Cortex-M (SWD ADIv5)**: STM32F1 (BluePill), STM32F4, STM32G0, STM32L4, STM32H5 (H503/H563 up to 2MB Flash), Puya PY32F0, Nordic nRF52840, RP2040, RP2350.
  * **WCH QingKe RISC-V**: CH32V003 (1-wire SWIO), CH32V203 / CH32V307 / CH592 (2-wire RVSWD/SDI).
* **Flashing Features**:
  * Automatic core detection, target IDCODE identification, and flash size parameter discovery.
  * Deterministic flash sector/page erase and RAM algorithm runner (`algo_runner`).
  * Direct flash read and memory verification (`GET /api/v1/programmer/read`).
  * Dedicated hardware reset (`nRST`) and boot control (`BOOT0`).

### 2. Mixed-Signal Instruments (Scope & Stimulus)
* **4-Channel Digital Oscilloscope**:
  * 500 kS/s single-channel / 125 kS/s 4-channel concurrent acquisition via RP2350 SAR ADC + DMA.
  * Hardware analog trigger engine: Rising edge, falling edge, voltage threshold, pre-trigger buffer.
  * Live waveform streaming (`GET /api/v1/oscilloscope/stream`) and automatic measurements ($V_{pp}, V_{rms}$, frequency, duty cycle).
  * Hardware front-end support for external Programmable Gain Amplifiers (TI PGA113 1x to 200x gain down to $5\,\text{mV}/\text{div}$).
* **Arbitrary Signal / Clock Generator**:
  * Dual-channel PWM stimulus on GPIO 10–29 via RP2350 PWM slices 0–7.
  * Frequency range from 1 Hz to 75 MHz with 0.1% to 100% duty cycle control.
  * Continuous and burst pulse modes for triggering external hardware.
* **Addressable LED Analyzer & Transmitter**:
  * Real-time 800 kHz WS2812 / NeoPixel transmission and capture.
  * Nanosecond timing verification for $T_{0H}$, $T_{1H}$, and reset latch periods.
* **Precision Glitch Injector**:
  * Cycle-accurate 150 MHz delay and glitch pulse generation for hardware fault injection research.

### 3. High-Speed Logic Analyzer
* **10 Channels**: Configurable digital capture across the flexible pin pool (`GPIO 20–29`).
* **Sampling Engine**: RP2350 PIO state machine + DMA circular buffer with fractional clock divider.
* **Trigger Engine**: Edge triggers (rising/falling/both), level qualifiers, and pattern matching.
* **Protocol Decoders**: Live firmware decoding for UART, I2C, SPI, CAN, 1-Wire, PWM, and I3C.
* **Export**: Streaming Server-Sent Events (`/api/v1/logic-analyzer/stream`) and standard `.vcd` export for PulseView and GTKWave.

### 4. AI-Native Model Context Protocol (MCP) Server
AgentLA embeds a native **Model Context Protocol (MCP)** server over USB virtual Ethernet (`http://10.10.10.10/mcp`), enabling LLMs (such as Google Antigravity, Claude Desktop, Cursor, and Cline) to directly control physical hardware in a closed loop:

```json
{
  "mcpServers": {
    "agentla": {
      "url": "http://10.10.10.10/mcp"
    }
  }
}
```

AI agents can autonomously flash firmware, measure voltages, monitor serial telemetry, trigger oscilloscope traces, and verify physical hardware assertions without human intervention.

### 5. Web Desktop OS
Served directly out of Flash ROM over USB CDC-NCM (`http://10.10.10.10` or `http://agentla.local`):
* Multi-window draggable desktop environment built with DaisyUI & Tailwind.
* Interactive Oscilloscope and Logic Analyzer waveform viewers.
* Target Programmer with hex inspector, one-click detection, and binary drag-and-drop flasher.
* Dynamic pin matrix and interactive bus console.
* LittleFS persistent flash storage for custom embedded Web Apps.

---

## Hardware REST API

All probe features are fully accessible via structured JSON REST endpoints:

| Endpoint | Method | Description |
| :--- | :--- | :--- |
| `/api/board/manifest` | `GET` | Dynamic board manifest, pinout matrix, and peripheral map |
| `/api/v1/programmer/target` | `GET` | Auto-detect connected ARM/RISC-V microcontroller |
| `/api/v1/programmer/read` | `GET` | Read target flash memory (`addr`, `len`) |
| `/api/v1/programmer/erase` | `POST` | Erase target flash sectors or mass erase |
| `/api/v1/programmer/flash` | `POST` | Program target flash memory with binary payload |
| `/api/v1/oscilloscope/data` | `GET` | Acquire single-shot 4-channel oscilloscope frame |
| `/api/v1/oscilloscope/stream` | `GET` | Continuous SSE waveform stream for oscilloscope |
| `/api/v1/generator/configure` | `POST` | Configure signal generator frequency, duty, and burst |
| `/api/v1/bridge/can` | `GET/POST`| Configure SocketCAN bitrate and transceiver mode |
| `/api/v1/bridge/can/termination`| `POST` | Toggle hardware $120\,\Omega$ differential termination |
| `/api/v1/logic-analyzer/stream`| `GET` | Live SSE digital waveform and decoder stream |

---

## Quick Start

### 1. Flashing AgentLA Firmware
1. Hold the **BOOTSEL** button on your Raspberry Pi Pico 2 / RP2350 board and plug it into USB.
2. Copy `agentla-pico2.uf2` to the mounted `RPI-RP2` volume.
3. The board reboots automatically as AgentLA (USB VID `1209:414d`).

### 2. Accessing the Web Desktop
Navigate to `http://10.10.10.10` or `http://agentla.local` in any browser.

### 3. Using Linux SocketCAN
```bash
# Bring up can0 at 500 kbps
sudo ip link set can0 up type can bitrate 500000

# Send and dump CAN frames
cansend can0 123#DEADBEEF
candump can0
```

### 4. Using Linux I2C
```bash
# Scan connected I2C devices on target bus
i2cdetect -y 1
```

### 5. Debugging with probe-rs
```bash
probe-rs list
# Recognizes: AgentLA RP2350 Probe (CMSIS-DAP)
```

---

## Building from Source

Prerequisites:
* Stable Rust toolchain with `thumbv8m.main-none-eabihf` target.
* `picotool` (v2.0+) for generating UF2 binaries.

```bash
# Clone the repository
git clone https://github.com/sun/AgentLA-rust.git
cd AgentLA-rust

# Install target
rustup target add thumbv8m.main-none-eabihf

# Build release firmware
cargo build --release --manifest-path firmware-rs/Cargo.toml

# Convert to UF2
picotool uf2 convert firmware-rs/target/thumbv8m.main-none-eabihf/release/firmware-rs agentla-pico2.uf2 --family rp2350-arm-s
```

---

## Credits & Attribution

AgentLA is built upon and inspired by the embedded Rust and hardware engineering communities. For full attribution of all directly compiled crates and upstream reference projects, see [CREDITS.md](CREDITS.md).

---

## License

Licensed under the **GNU General Public License**. See [LICENSE](LICENSE) for details.
