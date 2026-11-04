# AgentLA Hardware REST API Reference (`/api/v1`)

AgentLA exposes a zero-install, high-performance RESTful API over its virtual USB Ethernet interface (`http://10.10.10.10` or `http://agentla.local`). This API provides direct programmatic control over hardware pin muxing, bus bridges (I²C, SPI, UART, CAN, Addressable LEDs), target programming, logic analyzer acquisition, and probe telemetry.

All payloads use standard `application/json` formatting.

---

## 1. Hardware Profiles & Pin Multiplexing

### `GET /api/v1/profiles` (and legacy alias `GET /api/profiles`)
Retrieves all 32 hardware profile slots, their pin reservations, active state, and boot preference.

**Response**:
```json
{
  "active_slot": 1,
  "boot_slot": 1,
  "is_safe": false,
  "slots": [
    {
      "slot": 0,
      "name": "Safe State (Hi-Z)",
      "claimed_mask": 0,
      "is_active": false,
      "is_boot": false
    },
    {
      "slot": 1,
      "name": "SWD + UART Bridge",
      "claimed_mask": 3145731,
      "is_active": true,
      "is_boot": true
    }
  ]
}
```

### `POST /api/v1/profiles/apply`
Applies a profile by slot index or restores safe state.
- Body: `{"slot": 1}` or `{"safe": true}`

### `POST /api/v1/profiles/boot`
Configures persistent boot profile.
- Body: `{"slot": 1}`

### `POST /api/v1/profiles/save`
Stores an arbitrary profile definition into flash.
- Body: `{"slot": 2, "name": "Custom I2C", "claimed_mask": 12}`

### `DELETE /api/v1/profiles/{slot}`
Clears a profile slot back to defaults.

---

## 2. GPIO Pin Inspection & Direct SIO Control

### `GET /api/v1/pins`
Returns a compact real-time snapshot of all 30 GPIO lines.
- `p`: Pin number (0–29)
- `d`: Domain (`P` = Programmer GPIO 0–9, `A` = Adapter GPIO 10–19, `L` = Logic Analyzer GPIO 20–29)
- `m`: Function mode (`SIO`, `PIO`, `PWM`, `UART`, `I2C`, `SPI`, `HIZ`)
- `o`: Direction (`1` = Output, `0` = Input)
- `v`: Current digital logic level (`0` or `1`)

**Response**:
```json
{
  "status": "ok",
  "safe": false,
  "active_profile": "SWD + UART Bridge",
  "pins": [
    {"p":0,"d":"P","m":"PIO","o":1,"v":1},
    {"p":1,"d":"P","m":"PIO","o":0,"v":0}
  ]
}
```

### `GET /api/v1/pins/{gpio}`
Returns verbose status for an individual pin (e.g. `/api/v1/pins/10`).

### `POST /api/v1/pins/{gpio}/mode`
Configures pin mode and pull resistor.
- Body: `{"dir": "out", "pull": "up"}` (or `"pull": "down" | "none"`)

### `POST /api/v1/pins/{gpio}/write`
Sets digital logic output.
- Body: `{"val": 1}`

### `POST /api/v1/pins/claim` & `POST /api/v1/pins/release`
Hardware pin arbiter claim/release.
- Body: `{"pin": 10, "owner": "test"}`

---

## 3. Peripheral Bus Bridges

### `POST /api/v1/bus/i2c/scan`
Scans the I²C bus for responsive 7-bit peripheral addresses.
- Body: `{"bus": 0}` (optional)
- Response: `{"status": "ok", "devices": [32, 60], "count": 2}`

### `POST /api/v1/bus/i2c/transfer`
Atomic write and optional repeated-start read transaction.
- Body: `{"addr": 60, "write": "00AF", "read_len": 2}`
- Response: `{"status": "ok", "read": "1234"}`

### `POST /api/v1/bus/spi/transfer`
Full-duplex SPI bus transaction.
- Body: `{"tx": "9F000000", "mode": 0, "baud": 1000000}`
- Response: `{"status": "ok", "rx": "00204018"}`

### `POST /api/v1/bus/uart/write` & `GET /api/v1/bus/uart/read`
CDC UART bridge transmit and non-blocking receive.
- TX Body: `{"data": "Hello Target\n"}`
- RX Response: `{"status": "ok", "rx": "..."}`

### `POST /api/v1/bus/can/tx` & `GET /api/v1/bus/can/rx`
Hardware CAN bus frame transmission and reception.

---

## 4. Universal Addressable LED Driver (`/api/v1/bus/led/transmit`)

Supports high-speed streaming of pixel buffers to addressable LED strips up to 256 pixels, utilizing dedicated PIO hardware state machines (`neopixel.pio`) and high-speed SPI framing.

**Supported Protocols / Chipsets**:
- `ws2812` / `ws2812b`: 800 kHz 1-wire NRZ (GRB format)
- `ws2811`: 400 kHz 1-wire NRZ (RGB format)
- `sk6812`: 800 kHz 1-wire NRZ with dedicated White channel (GRBW / RGBW)
- `apa102` / `dotstar` / `sk9822`: 2-wire SPI clocked up to 10 MHz with 5-bit global brightness control

**Endpoint**: `POST /api/v1/bus/led/transmit`

**Payload Examples**:

1. **WS2812B 1-Wire Stream**:
```json
{
  "pin": 10,
  "chip": "ws2812b",
  "order": "grb",
  "hex": "FF000000FF000000FF"
}
```

2. **SK6812 RGBW 4-Channel Stream**:
```json
{
  "pin": 10,
  "chip": "sk6812",
  "order": "grbw",
  "hex": "000000FF"
}
```

3. **APA102 / DotStar SPI Stream**:
```json
{
  "chip": "apa102",
  "sck": 10,
  "mosi": 11,
  "brightness": 31,
  "hex": "FFFFFF00FF00"
}
```

---

## 5. Standalone Target Programmer Engine

### `GET /api/v1/programmer/target`
Interrogates target via SWD/SWIO/RVSWD and returns MCU family, Core ID, flash size, and security status.

### `POST /api/v1/programmer/reset`
Drives hardware `nRST` and `BOOT0` control lines.
- Body: `{"boot0": 1, "pulse_ms": 20}`

### `POST /api/v1/programmer/erase`
Mass-erases or sector-erases target flash memory.
- Body: `{"mass": true}` or `{"addr": 134217728, "len": 4096}`

### `POST /api/v1/programmer/flash`
Programs binary hex payload into target flash.
- Body: `{"addr": 134217728, "hex": "00400020..."}`

### `GET /api/v1/programmer/read`
Reads binary target flash using zero-RAM generator streaming.
- Query: `/api/v1/programmer/read?addr=0x08000000&len=1024`

---

## 6. Logic Analyzer & VCD Streaming

### `POST /api/v1/analyzer/arm`
Arms the 10-channel PIO sampling engine.
- Body: `{"sample_rate": 10000000, "trigger_pin": 20, "trigger_edge": "rising"}`

### `GET /api/v1/analyzer/status`
Queries acquisition progress (`idle`, `armed`, `capturing`, `complete`, `samples`).

### `POST /api/v1/analyzer/stop`
Forces acquisition stop.

### `GET /api/v1/analyzer/vcd`
Direct zero-RAM streaming export of the circular `captureBuffer` formatted as IEEE 1364 Value Change Dump (VCD). Fully compatible with Sigrok PulseView and GTKWave.
