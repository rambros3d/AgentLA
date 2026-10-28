## Context

AgentLA is an RP2350-based logic analyzer with dual-mode operation: on-device decoded mode for LLM agents and raw capture mode for Sigrok/PulseView compatibility. The firmware currently supports only passive bus analysis (capture what's on the wire). Agents can observe I2C/SPI/UART traffic but cannot initiate transactions — they cannot read a sensor register, write an I2C configuration byte, or send UART data.

RP2350 has spare hardware peripherals (I2C1 and UART1) and unused PIO state machines that can be repurposed for adapter mode. The USB CDC stack (TinyUSB) supports multiple CDC serial interfaces, allowing AgentLA to enumerate as two separate serial devices — one for analyzer mode, one for adapter mode — distinguished by USB product strings.

This change adds I2C and UART master adapter modes using hardware peripherals (not PIO), keeping them simple, reliable, and independent of analyzer mode.

## Goals / Non-Goals

**Goals:**
- AgentLA enumerates as two CDC devices: `AGENTLA Analyzer v1.0` and `AGENTLA Adapter v1.0`
- I2C1 hardware peripheral provides master read/write at 100kHz/400kHz
- UART1 hardware peripheral provides TX/RX at configurable baud rates
- Agent protocol extends with I2C_READ, I2C_WRITE, UART_WRITE, UART_READ commands
- AgentLA does NOT support SPI adapter mode in this change (deferred)

**Non-Goals:**
- SPI adapter mode (deferred to future change)
- I2C slave mode (only master)
- Bus arbitration / multi-master support
- Hardware changes (board revision) — I2C pull-ups require separate board design update
- Simultaneous adapter + analyzer operation on same CDC interface
- DMA-based I2C transfers (not needed for typical register reads/writes)

## Decisions

### D1: Hardware peripherals over PIO for I2C and UART

**Decision:** Use RP2350 I2C1 and UART1 hardware peripherals, not PIO bit-banging.

**Rationale:** I2C and UART are standard protocols with well-supported hardware in RP2350. Hardware peripherals handle clock stretching, ACK/NACK detection, framing, and baud rate generation automatically. PIO bit-banging would require manual timing management and is far less reliable for I2C (which has strict hold times).

**Alternatives considered:**
- PIO bit-bang I2C: Possible but fragile — clock stretching and timing margins are error-prone
- PIO bit-bang UART: Overkill — UART1 hardware peripheral handles all framing automatically
- Shared UART1 + I2C1 on pins already used for capture: Would conflict with capture mode

### D2: Fixed adapter pins, separate from capture pins

**Decision:** I2C uses GPIO0/GP1 (SCL/SDA), UART uses GPIO4/GP5 (TX/RX). These pins are different from capture pins (GPIO2-9).

**Rationale:** Fixed adapter pins don't conflict with analyzer mode capture pins. Agent knows which pins to use from the IDENTIFY response. No pin configuration negotiation needed.

**Alternatives considered:**
- Configurable adapter pins via CONFIGURE command: More flexible but adds complexity and error surface. Fixed pins are simpler and more predictable.
- Sharing capture pins for adapter: Would create contention between analyzer input and adapter output.

### D3: Two USB CDC devices (not mode-switching on one port)

**Decision:** AgentLA enumerates as two distinct CDC serial devices with different USB product strings.

**Rationale:** Mode-switching on a single port adds protocol complexity (agent must switch context). Two CDC devices are naturally separated: the agent opens the analyzer port for capture and the adapter port for bus transactions. No mode awareness needed at the protocol level.

**Alternatives considered:**
- Single CDC, mode-switch command: Simpler USB enumeration but agent must manage mode state machine
- Single CDC with multiplexed commands: Agent must distinguish between analyzer and adapter commands — fragile

### D4: I2C pull-ups require board hardware change

**Decision:** I2C master mode requires external 4.7kΩ pull-ups on SDA and SCL. Document this as required but do not add them to the current board revision.

**Rationale:** RP2350 internal pull-ups (~40kΩ) are too weak for reliable I2C above 100kHz. External pull-ups are a standard requirement for any I2C master.

**Alternatives considered:**
- Enable internal pull-ups as fallback: Works at very low speeds only, unreliable for sensor communication
- Include pull-ups on current board revision: Requires PCB respin

### D5: IDENTIFY returns adapter role and capabilities

**Decision:** IDENTIFY on the adapter CDC returns a string including role and capabilities: `AGENTLA v1.0 RP2350 ADAPTER I2C+UART`.

**Rationale:** Agent auto-discovers device capabilities from IDENTIFY without needing to probe each command. Clear, human-readable format consistent with analyzer IDENTIFY.

**Alternatives considered:**
- Separate QUERY command: Adds protocol overhead, IDENTIFY is the standard discovery mechanism
- USB descriptor enumeration: OS-level but not portable across all platforms

## Risks / Trade-offs

- **[Risk] I2C bus contention** → Mitigation: Agent software should check bus idle before starting transaction; hardware I2C1 handles NACK but not arbitration
- **[Risk] No I2C pull-ups on current board** → Mitigation: Document as required; internal pull-ups work for very basic testing only
- **[Risk] UART baud rate mismatch** → Mitigation: Validate baud rate in UART_WRITE command; support common rates (9600-230400)
- **[Risk] USB CDC enumeration order non-deterministic** → Mitigation: Agent should identify by product string, not port order
- **[Trade-off] Fixed adapter pins limit board flexibility** → Acceptable: Adapter pins are different from capture pins, so no conflict; future revision can add more adapter pin options

## Migration Plan

1. Flash new firmware with adapter mode support
2. Agent connects to `AGENTLA Adapter` CDC device (product string)
3. Agent sends IDENTIFY to confirm role and capabilities
4. Agent sends I2C_READ/I2C_WRITE/UART_WRITE/UART_READ as needed
5. No migration needed for existing analyzer firmware or SDK usage

## Open Questions

1. **Should adapter mode support GPIO direction configuration?** (e.g., allowing agent to set a GPIO high/low to drive an LED or read a button)
2. **Should I2C clock stretch timeout be configurable?** (some sensors hold SCL low for extended periods)
3. **Should UART adapter support hardware flow control (RTS/CTS)?** (needed for some high-speed UART devices)
4. **How does the agent discover the adapter CDC device?** (product string matching, USB enumeration scan, or explicit configuration)