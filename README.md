# AgentLA

AgentLA is an open-source hardware debugger and bring-up tool for embedded boards.
Plug it in, open a browser, and inspect, program, or interact with your target —
no host software or drivers required.

## Features
- Target flashing over SWD (STM32, Nordic, WCH, CH32)
- 10-channel logic analyzer with protocol decoding (UART, I²C, SPI, PWM)
- GPIOs, I2C / SPI bus masters, UART terminal
- Web-based desktop interface (WinBox) for quick control
- Direct USB access (virtual Ethernet + serial console)
- Support for RP2040 / RP2350 boards

## Quick Start
1. Flash the latest `AgentLA.uf2` to your Pico (hold BOOTSEL and drag-drop).
2. Plug in USB and let the network interface obtain an IP.
3. Open: `http://agentla.lab` (or `http://192.168.7.1`)
4. Use the browser desktop to interact with targets and tools.

## Updating firmware
Put the probe in BOOTSEL mode (or send `BOOTSEL` on CDC port) and copy the newer `AgentLA.uf2`.
It rebuilds via CMake (see `CMakeLists.txt`), and will flash automatically; data remains user configurable unless you wipe the target.

You can also update over the USB network (hands-free) whenever not in BOOTSEL.

## Support and development
- Firmware source in `firmware/src/` (C + PIO).
- Web UI is embedded (`firmware/html/*`).
- OpenSpec changes are tracked under `openspec/changes/`.
- Report issues via manual tests validated through `test/` and `note.txt`.