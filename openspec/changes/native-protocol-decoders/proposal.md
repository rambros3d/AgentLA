# Proposal: Native Protocol Decoders (Phases 1–5)

## Why
While AgentLA currently supports basic in-firmware decoders for I2C, SPI, UART, and PWM, modern embedded, industrial, and automotive development relies on specialized protocols (CAN-FD, Modbus RTU, DMX-512, Addressable LEDs, 1-Wire, and ARM SWD/SWO trace). Siling Probe features dedicated on-board physical transceivers (CAN-FD TCAN334 and RS-485 SN65HVD3082) and 16-channel 100/150 MS/s capture pins that currently require external tools like PulseView or Sigrok for higher-layer decoding.

Implementing native, zero-heap C decoders directly in the RP2350 firmware provides immediate real-time protocol analysis over the serial CLI, WebUI Server-Sent Events (`/events`), and interactive browser Canvas waveforms without external software dependencies or PC host bottlenecks.

## What Changes
- **Phase 1: Automotive & Industrial Core**:
  - Add native **CAN 2.0B / CAN-FD Decoder (`can-fd-protocol-decoder`)**: Bit de-stuffing, nominal/data phase bit timing ($75\%-80\%$ sample point), 11/29-bit Arbitration IDs, BRS, DLC up to 64 bytes, and hardware CRC-15/17/21 validation.
  - Add native **Modbus RTU / RS-485 Decoder (`modbus-rtu-protocol-decoder`)**: $3.5\times t_{\text{char}}$ idle line inter-frame delimitation, Slave ID (1–247), Function Codes (`0x01`–`0x06`, `0x0F`, `0x10`), Exception code handling, and CRC-16 Modbus polynomial (`0xA001`) verification.
  - Add native **DMX-512 Decoder (`dmx512-protocol-decoder`)**: Break ($\ge 88\,\mu\text{s}$), Mark-After-Break (MAB $\ge 8\,\mu\text{s}$), Start Code `0x00`, and channel level extractions ($0-255$) across channels 1–512.
- **Phase 2: Universal Addressable LED Engine**:
  - Add native **Universal LED Decoder (`addressable-led-decoder`)**:
    - **Single-Wire PWM/NRZ Engine**: WS2812B, WS2811, SK6812 (RGB & RGBW), WS2813/15 (backup line), UCS1903, and TM1814 with pulse-width timing jitter/violation diagnostics ($T_{\text{high}}$ alarms).
    - **Two-Wire Clocked SPI Engine**: APA102 / DotStar, SK9822, WS2801, and LPD8806 with 5-bit Global Brightness multiplier and 24-bit per-pixel RGB extraction.
- **Phase 3: Sensors, Automotive LIN & Next-Gen Buses**:
  - Add native **1-Wire / Dallas Decoder (`onewire-dallas-decoder`)**: Reset/Presence pulse measurement, 64-bit ROM ID registration with DOW CRC, and automatic **DS18B20 temperature extraction in Celsius**.
  - Add native **LIN Bus Decoder**: Break, Sync `0x55`, Protected ID (PID) with parity, and classic/enhanced checksums.
  - Add native **MIPI I3C SDR Decoder (`mipi-i3c-sdr-decoder`)**: Broadcast Header `0x7E`, Common Command Codes (CCC: `ENTDAA`, `SETDASA`, `GETPID`, etc.), and Transition `T` bit parity validation.
- **Phase 4: Embedded Silicon Debug & Trace**:
  - Add native **ARM SWD Protocol Sniffer (`arm-debug-trace-decoder`)**: SWCLK/SWDIO transaction monitor, DP/AP register reads/writes (DPIDR, CTRL/STAT, SELECT, TAR, DRW), 3-bit ACK, and parity verification.
  - Add native **ARM SWO / ITM Trace Printf Decoder**: Reconstructs asynchronous UART/Manchester trace packets from Cortex-M SWO into human-readable ASCII debug console text.
  - Add native **SPI Flash Command Interpreter**: Translates raw SPI bytes into human-readable NOR flash commands (JEDEC ID `0x9F`, Sector Erase `0x20`, Page Program `0x02`, Fast Read `0x0B`).
- **Phase 5: WebUI Visual Overlays & Streaming Integration**:
  - Add **WebUI Visual Overlays (`webui-protocol-overlays`)**: Interactive Canvas waveform annotation bubbles, colored CSS swatches rendering actual RGB LED pixel colors, and live Server-Sent Events (`/events`) streaming.

## Capabilities

### New Capabilities
- `can-fd-protocol-decoder`: In-firmware CAN 2.0B and CAN-FD bitstream decoder with bit-destuffing, 11/29-bit Arbitration IDs, DLC (up to 64 bytes), and CRC15/17/21 validation.
- `modbus-rtu-protocol-decoder`: In-firmware Modbus RTU packet decoder with $3.5\times t_{\text{char}}$ idle line framing, Function Codes 0x01–0x10, and CRC-16 Modbus validation.
- `dmx512-protocol-decoder`: In-firmware DMX-512 stage lighting packet decoder validating Break, MAB, Start Code, and channel slots 1–512.
- `addressable-led-decoder`: Universal in-firmware addressable LED decoder for 1-wire PWM (WS2812B, SK6812 RGBW, WS2811, WS2813/15, UCS1903) and 2-wire clocked SPI (APA102/DotStar, SK9822, WS2801) with timing jitter diagnostics.
- `onewire-dallas-decoder`: In-firmware 1-Wire Dallas decoder validating Reset/Presence timing, 64-bit ROM registration, and automatic DS18B20 temperature extraction.
- `mipi-i3c-sdr-decoder`: In-firmware MIPI I3C SDR decoder parsing Broadcast Header `0x7E`, Common Command Codes (CCC), Dynamic Address Assignment (`ENTDAA`), and transition `T` parity.
- `arm-debug-trace-decoder`: In-firmware ARM SWD packet sniffer and SWO ITM trace decoder for realtime debug console text.
- `webui-protocol-overlays`: Interactive WebUI Canvas protocol visual overlays with colored bubbles, CSS RGB swatches for LED pixels, and live SSE streaming logs.

### Modified Capabilities
*(None - existing I2C/SPI/UART decoders remain fully backward compatible).*

## Impact
- **Firmware Engine**:
  - `firmware/src/analyzer/decode.h`: Refactor `DECODE_EVENT` into compact union-based `DECODER_EVENT_V2` (24 bytes).
  - New modular source files: `decode_can.c`, `decode_modbus.c`, `decode_dmx.c`, `decode_led.c`, `decode_1wire.c`, `decode_i3c.c`, `decode_swd.c`, `decode_swo.c`.
- **Command Surface**:
  - `firmware/src/main.c` & `webui_command_bridge.c`: Extend `CAPTURE mode=DECODED protocol=<CAN|MODBUS|DMX|LED_1W|LED_2W|1WIRE|I3C|SWD>` CLI grammar.
- **Web Interface**:
  - `firmware/html/app.js` & `web/demo/app.js`: Add visual Canvas annotation renderers and protocol options to `.decoder-menu`.
