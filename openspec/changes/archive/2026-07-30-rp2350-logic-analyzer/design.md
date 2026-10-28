## Context

AgentLA is an RP2350-based logic analyzer designed for LLM agent integration. It must serve two distinct users: humans debugging electronics via Sigrok/PulseView, and AI agents reasoning about signals through structured text. The firmware is forked from gusmanb/logicanalyzer (GPL-3.0, C/Pico SDK), which provides proven RP2350 capture with Errata E9 workarounds. The host is a Python SDK that agents call programmatically.

The core tension: humans want waveforms and visual decoders; agents want structured text they can reason about. Both must coexist on the same hardware.

## Goals / Non-Goals

**Goals:**
- Dual-mode operation: raw capture for Sigrok + on-device decoded mode for agents
- Text-based command protocol that LLM agents can parse without training
- On-device protocol decode for I2C, SPI, UART, PWM via ARM Cortex-M33 at 150 MHz
- Full Sigrok/PulseView compatibility in raw mode
- 8 channels minimum, expandable to 32 via GPIO
- Sample rates from 100 kHz to 100 MHz (50 MHz realistic for 8ch)
- Edge and pattern triggering with pre/post-trigger buffers

**Non-Goals:**
- Analog voltage measurement (future expansion, not v1)
- On-device Sigrok decoder integration (ARM decoders are purpose-built)
- Multi-board synchronization
- WiFi/Bluetooth connectivity
- Web UI (agents don't need it, humans have Sigrok)

## Decisions

### D1: Fork gusmanb/logicanalyzer vs build fresh

**Decision:** Fork gusmanb/logicanalyzer.

**Rationale:** Proven RP2350 support, Errata E9 solved on PCB v6.0, PIO capture engine + DMA + triggers working, level shifting PCB design ready. Saves 2-3 months of hardware bring-up.

**Alternatives considered:**
- Fork pico-coder/sigrok-pico: Better Sigrok integration but RP2040-focused, would need porting
- Fork μLA: Rust (team unfamiliar), Apache license (cleaner) but less mature RP2350 support
- Build from scratch: Maximum control but 3-6 month timeline

### D2: PIO capture + ARM decode vs PIO-only decode

**Decision:** PIO handles raw capture only. ARM Cortex-M33 at 150 MHz handles protocol decode.

**Rationale:** PIO state machines have 32 instruction limit. I2C decode (start/stop/ACK/address/data/error detection) requires ~40+ instructions - doesn't fit. SPI byte framing fits (~12 instructions) but multi-byte state tracking doesn't. ARM at 150 MHz has headroom for all protocols simultaneously at full sample rate.

**Alternatives considered:**
- PIO-only decode: Limited to simple protocols, no I2C full decode, can't track multi-byte state
- Chained PIO SMs: Complex synchronization, still limited instruction count
- DMA interrupt-driven decode: ARM interrupt fires on DMA completion, decodes in batches - **this is what we use**

### D3: Text-based protocol vs binary protocol

**Decision:** Text-based command protocol over USB CDC serial.

**Rationale:** LLM agents reason about text natively. Binary protocols require bit manipulation that confuses models. Text is debuggable with serial terminals. Human-readable error messages help agents self-correct.

**Alternatives considered:**
- Binary protocol: More efficient, but harder for agents to parse
- JSON-only: More structured but verbose for simple commands
- Register-level API: Too low-level, agents need abstraction

### D4: USB CDC serial vs bulk endpoints

**Decision:** USB CDC serial (virtual COM port).

**Rationale:** Universal driver support, works with Python's pyserial, compatible with Sigrok's serial transport, debuggable with any terminal emulator. ~600 KB/s sustained is sufficient for 8ch at 1 MHz sample rate (1 MB/s raw, compressible).

**Alternatives considered:**
- USB bulk: Higher throughput (~900 KB/s) but more complex driver setup
- USB HID: Universal but throughput too low
- UART: Too slow for raw capture data

### D5: Protocol decode architecture

**Decision:** Event-driven decoder with per-protocol handler functions.

**Architecture:**
```
PIO samples pins → DMA fills SRAM buffer → DMA IRQ fires →
ARM reads buffer → event loop calls decoder per channel →
decoder maintains state machine → outputs structured text →
USB serial sends to host
```

**Decoder state machines:**
- I2C: SCL edge counter, SDA sampling, ACK bit detection, 7-bit address shift register
- SPI: CS assertion detection, clock edge counting, MOSI/MISO bit collection
- UART: Start bit falling edge detection, baud rate timing, parity calculation
- PWM: Two timestamp captures per cycle, period/duty calculation

### D6: Memory layout for capture buffer

**Decision:** Dual-buffer with ping-pong DMA.

**Rationale:** RP2350 has 520 KB SRAM. At 8ch × 100 MHz × 1 second = 100 MB/s data rate - exceeds USB bandwidth. Strategy:
- Capture buffer: 256 KB SRAM (split into 2×128 KB halves for ping-pong)
- Capture at full rate into SRAM via DMA
- Simultaneously stream to USB at whatever rate USB can sustain
- On-device decode mode: ARM processes buffer in-place after capture completes
- Raw mode: Stream buffer contents over USB as they fill

**Trade-off:** At high sample rates, capture time is limited by SRAM. 100 MHz × 8ch = 100 MB/s, so ~1.2 seconds max. At 10 MHz × 8ch, ~12 seconds. Acceptable for logic analysis use cases.

### D7: Command protocol design

**Decision:** Line-based text protocol with `<COMMAND> <ARGS>` syntax and `<RESPONSE>` format.

**Protocol sketch:**
```
Host → Device: IDENTIFY\r\n
Device → Host: AGENTLA v1.0 RP2350 32CH 100MHZ\r\n

Host → Device: CONFIGURE channels=0,1,2,3 rate=10000000 trigger=RISING pin=0\r\n
Device → Host: OK configured 4 channels at 10000000 Hz\r\n

Host → Device: CAPTURE samples=4096 mode=RAW\r\n
Device → Host: CAPTURING...\r\n
Device → Host: RAW 00 FF 3C ... (hex-encoded pin states)\r\n
Device → Host: DONE samples=4096 time=0.0004096\r\n

Host → Device: CAPTURE samples=4096 mode=DECODED protocol=I2C sda=0 scl=1\r\n
Device → Host: CAPTURING...\r\n
Device → Host: I2C START\r\n
Device → Host: I2C ADDR 0x48 R\r\n
Device → Host: I2C ACK\r\n
Device → Host: I2C DATA 0x7F\r\n
Device → Host: I2C STOP\r\n
Device → Host: DONE decoded=12 transactions=3\r\n
```

### D8: Python SDK architecture

**Decision:** Synchronous API with pyserial backend, context manager pattern.

```python
import agentla

with agentla.connect("/dev/ttyACM0") as device:
    info = device.identify()
    print(f"Found: {info.model} {info.channels}ch")
    
    device.configure(channels=[0,1,2,3], sample_rate=10_000_000)
    
    # Raw mode (Sigrok-compatible)
    data = device.capture_raw(samples=4096, trigger={"pin": 0, "edge": "rising"})
    
    # Decoded mode (agent-friendly)
    events = device.capture_decoded(
        protocol="I2C", 
        sda=0, scl=1, 
        samples=4096
    )
    for event in events:
        print(event)  # I2CEvent(type='START', timestamp=0.0)
```

**Alternatives considered:**
- Async API: More complex, agents don't need concurrent operations
- Callback-based: Harder for agents to reason about sequential logic

## Risks / Trade-offs

- **[Risk] Errata E9 GPIO latchup** → Mitigation: Fork gusmanb's PCB v6.0 with proven workarounds; document A4 silicon revision check
- **[Risk] USB bandwidth limits decode throughput** → Mitigation: On-device decode reduces data to structured text (1000:1 compression); raw mode streams at USB-sustainable rate
- **[Risk] GPL-3.0 license from gusmanb** → Mitigation: Accept GPL for firmware; Python SDK can be separate license; document clearly
- **[Risk] PIO capture limited by SRAM** → Mitigation: 256 KB buffer at 10 MHz gives ~12 seconds; document max capture times per sample rate
- **[Risk] ARM decode latency at high sample rates** → Mitigation: DMA interrupt-driven batching; 150 MHz M33 has ~67 ns per instruction - sufficient for 100 kHz protocols
- **[Trade-off] Text protocol slower than binary** → Acceptable: agents need readability; USB bandwidth is the real bottleneck anyway
- **[Trade-off] No analog measurement in v1** → Acceptable: keep scope focused; expansion header reserved for future ADC channels

## Migration Plan

No migration - this is a new project forking an existing codebase. The gusmanb firmware serves as the starting point with working capture, DMA, and triggers. Our additions (protocol decoders, command parser, dual-mode) are layered on top.

## Open Questions

1. **License choice for the agentla project itself** - GPL-3.0 to match gusmanb? Or Apache-2.0 for broader adoption?
2. **Minimum viable sample rate for I2C decode** - 100 kHz standard mode requires 1 MHz sample rate minimum; 400 kHz fast mode needs 4 MHz+
3. **How many simultaneous decoded protocols** - Can ARM handle SPI + I2C + UART simultaneously at full rate?
4. **Trigger in decoded mode** - Can we trigger on protocol events (e.g., "start capture when I2C address 0x48 appears") or only pin-level triggers?
