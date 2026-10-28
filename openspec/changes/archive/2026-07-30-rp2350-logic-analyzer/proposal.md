## Why

LLM agents need a bridge to the physical world of electronics debugging. Existing logic analyzers (gusmanb/logicanalyzer, sigrok-pico, μLA) all assume a human looking at waveforms on a host PC - they stream raw data and rely on host-side Sigrok decoders. None provide on-device protocol decode with a structured, agent-readable output format. AgentLA solves this by building an RP2350-based logic analyzer that supports both raw capture (for human/Sigrok use) AND on-device decoded mode (for LLM agents), with a text-based command protocol that agents can reason about.

## What Changes

- Fork gusmanb/logicanalyzer firmware as the base (proven RP2350 support, Errata E9 workarounds, PIO capture engine, DMA, triggers, level shifting PCB)
- Add ARM-core protocol decoders for I2C, SPI, UART, and PWM measurement running on the Cortex-M33 at 150 MHz
- Implement dual-mode operation: `RAW` mode streams pin states for host-side decode, `DECODED` mode performs on-device protocol decode and outputs structured text
- Design a text-based command protocol over USB CDC serial that LLM agents can parse and reason about
- Build a Python `agentla` SDK that wraps the serial protocol into clean function calls for agent integration
- Keep full Sigrok/PulseView compatibility for human use in raw mode

## Capabilities

### New Capabilities
- `capture-engine`: PIO-based digital capture with DMA, configurable sample rates (100 kHz - 100 MHz), 8-32 channels, edge/pattern triggers, pre/post-trigger sampling
- `protocol-decode`: ARM-core decoders for I2C (start/stop/ACK/address/data), SPI (CS/clock/MOSI/MISO), UART (start/data/parity/stop), PWM (frequency/duty/period measurement)
- `agent-protocol`: Text-based command protocol over USB CDC serial with IDENTIFY, CONFIGURE, CAPTURE, DECODE, MEASURE, STATUS commands
- `python-sdk`: Python `agentla` package providing programmatic API for LLM agents to configure, capture, decode, and query the device
- `raw-capture-mode`: Raw pin state streaming compatible with Sigrok/PulseView for human visualization and host-side decode

### Modified Capabilities
(none - this is a new project)

## Impact

- **Hardware**: RP2350 (Pico 2) board, custom PCB with level shifters (3.3V/5V/ext Vref), probe cables
- **Firmware**: C with Pico SDK, forked from gusmanb/logicanalyzer. PIO programs for capture, C decoders for protocols, USB CDC serial command parser
- **Host Software**: Python SDK, compatible with existing Sigrok/PulseView in raw mode
- **Dependencies**: pico-sdk, TinyUSB (USB stack), optional: libsigrokdecode (for extending host-side decoders)
- **License**: TBD (gusmanb is GPL-3.0, μLA is Apache-2.0, Bus Pirate is MIT)
