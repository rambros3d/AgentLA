---
name: agentla-analyzer
description: High-speed digital logic analyzer, bus protocol sniffer, and packet stimulus specialist. Enforces Pin Arbiter reservation contracts, decodes multi-protocol buses (I2C, SPI, UART, CAN, 1-Wire), controls the 120Ω CAN termination relay, and injects test vectors.
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  protocols_supported: ["I2C", "SPI", "UART", "CAN-2.0B / CAN-FD", "1-Wire", "DMX512", "WS2812"]
  tools_required: ["agentla-mcp"]
---

# Bus Protocol & Logic Analyzer Specialist (`agentla-analyzer`)

This skill guides AI agents through high-speed digital signal acquisition, protocol decoding, and hardware packet stimulus using AgentLA's logic analyzer and hardware bridges.

---

## Signal Capture & Analysis Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 1. PIN ARBITER CLAIM                                                        │
│    pins_claim (bitmask, owner, description)                                 │
│    [Acquire probe channels D0..D7; verify no conflicts with UART or SWD]   │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 2. PHYSICAL SIGNAL CONDITIONING                                             │
│    can_set_termination / i2c_pullup                                        │
│    [Enable internal 120Ω CAN termination relay or I2C pull-ups if needed]  │
└──────────────────────────────────────┬──────────────────────────────────────┘
                                       │
                                       ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 3. HIGH-SPEED SAMPLING & PROTOCOL SNIFFING                                  │
│    logic_capture_trigger ──▶ mcp_sniff_bus / can_sniff                      │
│    [Sample at up to 100MS/s; decode bus transactions into structured JSON] │
└───────────────────┬─────────────────────────────────────┬───────────────────┘
                    │ Passive Sniffing Complete           │ Active Stimulus Required
                    ▼                                     ▼
┌──────────────────────────────────────┐  ┌───────────────────────────────────┐
│ 4a. DECODE ANALYSIS & REPORT         │  │ 4b. INTERACTIVE PACKET INJECTION  │
│     Extract timestamps, ACK/NACK,    │  │     can_send / uart_command       │
│     hex payloads, frame errors.      │  │     spi_transfer / i2c_write      │
└───────────────────┬──────────────────┘  └─────────────────┬─────────────────┘
                    │                                       │
                    └───────────────────┬───────────────────┘
                                        │
                                        ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│ 5. TEARDOWN & PIN ARBITER RELEASE                                           │
│    pins_release (bitmask)                                                   │
│    [Release ownership to restore floating/safe state for other subsystems]  │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Safety Invariants & Arbitration Rules

> [!CAUTION]
> **Pin Arbiter Contract**: NEVER touch probe channels D0..D7 or dedicated bridge lines without acquiring ownership via `pins_claim`. The Pin Arbiter enforces mutual exclusion between logic analyzer capture, CMSIS-DAP debug lines, and serial UART bridges. If `pins_claim` returns an error, identify the conflicting owner before retrying.

1. **Mandatory Teardown**: Always release claimed pins with `pins_release` upon exit, even when an error occurs or a timeout triggers.
2. **CAN Bus Termination Care**: Do NOT engage the internal $120\,\Omega$ termination relay (`can_set_termination`) if the physical bus already has two terminating resistors installed. Doing so drops line impedance to $40\,\Omega$, overloading transceiver drivers.
3. **Sampling Nyquist Invariant**: Always configure the logic analyzer sample rate to at least $4\times$ (ideally $10\times$) the maximum expected bus baud rate. For example, a $400\text{ kHz}$ Fast-Mode I2C bus requires $\ge 4\text{ MS/s}$.

---

## Step-by-Step Analysis Protocol

### Phase 1: Pin Arbiter Acquisition

1. Compute the channel bitmask:
   - Channel D0: `1 << 0` = `1`
   - Channel D1: `1 << 1` = `2`
   - Channels D0 & D1: `1 | 2` = `3`
   - All 8 channels (D0..D7): `0xFF` = `255`
2. Claim pins using MCP tool `pins_claim`:
   ```json
   {
     "mask": 3,
     "owner": "analyzer",
     "description": "Capture I2C SDA and SCL transactions"
   }
   ```
3. If an error is returned:
   - Check which subsystem holds the conflicting pin (`uart`, `swd`, or `gpio`).
   - If appropriate, release or coordinate with the holding subsystem before re-claiming.

---

### Phase 2: Bus Conditioning & Relay Control

1. **CAN Bus Termination**:
   - For isolated benchtop setups where AgentLA is an end node:
     ```json
     { "enabled": true }
     ```
     via `can_set_termination`.
2. **I2C Bus Pull-Ups**:
   - Ensure external pull-ups ($2.2\text{k}\Omega\text{--}4.7\text{k}\Omega$) exist or enable internal pull-ups on target MCU.

---

### Phase 3: Protocol Sniffing & Triggered Logic Capture

#### A. Multi-Protocol Sniffing (`mcp_sniff_bus`)
1. Call MCP tool `mcp_sniff_bus` with desired protocol:
   ```json
   {
     "protocol": "i2c",
     "timeout_ms": 1000,
     "max_events": 50
   }
   ```
2. Inspect returned transactions:
   - Verify start condition (`S`), 7-bit slave address, read/write bit (`R`/`W`), ACK/NACK responses, and payload bytes.

#### B. High-Speed Triggered Logic Capture (`logic_capture_trigger`)
1. Configure trigger edge and buffer size:
   ```json
   {
     "channel_mask": 255,
     "sample_rate_hz": 10000000,
     "trigger_channel": 0,
     "trigger_edge": "falling",
     "samples_before": 1000,
     "samples_after": 9000
   }
   ```
2. Read back transitions and timestamp deltas to measure clock frequencies, pulse widths, and rise/fall jitter.

---

### Phase 4: Interactive Packet Injection

Stimulate the target device to test its response under live stimuli:

1. **CAN Frame Transmission (`can_send`)**:
   ```json
   {
     "id": 290,
     "data_hex": "DEADBEEF01020304",
     "extended": false
   }
   ```
2. **Serial UART Stimulus (`uart_command`)**:
   ```json
   {
     "tx_data": "AT+STATUS?\r\n",
     "baud": 115200,
     "timeout_ms": 500
   }
   ```
3. **SPI Full-Duplex Transfer (`spi_transfer`)**:
   ```json
   {
     "mosi_hex": "03000000",
     "cs_pin": 17,
     "speed_khz": 2000
   }
   ```

---

### Phase 5: Clean Teardown

1. Release the claimed channels:
   - Call MCP tool `pins_release`:
     ```json
     { "mask": 3 }
     ```
2. Confirm all pins are returned to high-impedance safe state.

---

## Output Reporting Standard

```tap
TAP version 13
1..4
ok 1 - Pin Arbiter Claim (Channels D0..D1 reserved)
ok 2 - Bus Conditioning (CAN 120 Ohm relay engaged)
ok 3 - Protocol Sniffing (14 I2C transactions captured, all ACKed)
ok 4 - Pin Arbiter Teardown (Pins released cleanly)
```

| Channel | Signal | Protocol | Events Detected | Errors / NACKs | Verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **D0** | SDA | I2C (Address `0x68`) | 14 Read Transactions | 0 NACKs | **PASS** |
| **D1** | SCL | I2C Clock ($400\text{ kHz}$) | Clock stable, 50% duty | 0 Clock Stretches | **PASS** |
| **CAN-H / L** | Differential | ISO 11898-2 ($500\text{ kbps}$) | 32 Data Frames | 0 Stuff Errors | **PASS** |
