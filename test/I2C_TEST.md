# I2C Adapter Mode Test Wiring

## Purpose
Test the RP2350 AgentLA I2C adapter mode using an ESP32 as an I2C slave.

## Wiring Diagram

```
RP2350 (AgentLA Adapter)     ESP32 (I2C Slave)

GPIO0  [SCL]  ───────────── GPIO22 [SCL]
GPIO1  [SDA]  ───────────── GPIO21 [SDA]
  3V3         ──┤4.7kΩ├── VCC (3.3V)    (pull-up on SDA)
  3V3         ──┤4.7kΩ├── VCC (3.3V)    (pull-up on SCL)
GND           ───────────── GND
```

## ESP32 Setup

Flash the ESP32 with `i2c_slave.py`:
```bash
# On the ESP32 (via esptool or Thonny):
micropython i2c_slave.py
```

The ESP32 acts as I2C slave address `0x48` with register map:
- `0x00`: temperature high byte (mock value `0x1A` = 26°C)
- `0x01`: temperature low byte (`0x40`)
- `0x02`: config byte (read/write)

## AgentLA Commands

### 1. I2C Read (read 2 bytes from register 0x00 of slave 0x48)
```
I2C_READ addr=0x48 reg=0x00 len=2
```
Expected response: `I2C DATA 1A,40`

### 2. I2C Write (write 1 byte to register 0x02 of slave 0x48)
```
I2C_WRITE addr=0x48 reg=0x02 data=01
```
Expected response: `OK I2C WRITE 1 bytes`

### 3. I2C Read back the config byte
```
I2C_READ addr=0x48 reg=0x02 len=1
```
Expected response: `I2C DATA 01`

### 4. I2C bus clear (if bus gets stuck)
```
I2C_CLEAR
```
Expected response: `OK I2C BUS CLEARED`

## Python SDK Test

```python
import agentla

with agentla.connect() as device:
    info = device.identify()
    assert info.is_adapter, "Device is not in adapter mode"

    _, i2c = device.adapter()

    # Read temperature registers
    data = i2c.read(addr=0x48, reg=0x00, length=2, timeout_ms=100)
    print(f"Temperature: 0x{data.hex()}")

    # Write to config register
    i2c.write(addr=0x48, reg=0x02, data=b"\x01", timeout_ms=100)

    # NACK test (non-existent device)
    try:
        i2c.read(addr=0x7F, reg=0x00, length=1, timeout_ms=100)
    except agentla.DeviceError as e:
        print(f"NACK handled: {e}")
```

## Alternative I2C Devices

Any I2C slave device with address 0x48 can be used instead of the ESP32:
- TMP102 temperature sensor (address 0x48)
- EEPROM (AT24Cxx, address configurable)
- Any I2C sensor with register read/write capability

## Notes

- **External pull-ups required**: 4.7kΩ on SDA and SCL to 3.3V (not on current board)
- **I2C speed**: Default 100kHz; internal pull-ups (~40kΩ) only work at very low speeds
- **Clock stretching**: RP2350 I2C hardware handles clock stretching internally with a 100ms timeout