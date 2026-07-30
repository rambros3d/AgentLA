"""ESP32 I2C Slave for RP2350 AgentLA adapter testing.

Wire up:
  RP2350 GPIO0 (SCL) → ESP32 GPIO22 (SCL)
  RP2350 GPIO1 (SDA) → ESP32 GPIO21 (SDA)
  3.3V → 4.7kΩ → SDA
  3.3V → 4.7kΩ → SCL

ESP32 I2C slave address: 0x48 (same as TMP102 temp sensor address)
Register map:
  0x00: temperature high byte
  0x01: temperature low byte
  0x02: config byte (read/write)
"""

from machine import I2C, Pin

i2c = I2C(0, scl=Pin(22), sda=Pin(21), freq=100000)

SLAVE_ADDR = 0x48
registers = bytearray(0x10)
registers[0x00] = 0x1A  # temperature high byte (mock: 26 degrees)
registers[0x01] = 0x40  # temperature low byte
registers[0x02] = 0x00  # config byte

def handle_i2c_slave(event, data):
    if event == I2C.EVENT_SLAVE_RECEIVED:
        addr = i2c.is_master()
        reg = data[0] if len(data) > 0 else 0
        print(f"I2C WRITE slave={SLAVE_ADDR:#04x} reg={reg:#04x} data={data[1:].hex() if len(data) > 1 else 'none'}")
        if len(data) > 1:
            registers[reg] = data[1]
    elif event == I2C.EVENT_SLAVE_REQUEST:
        reg = registers[0]  # last register accessed
        print(f"I2C READ slave={SLAVE_ADDR:#04x} reg={reg:#04x} data={registers[reg]:#04x}")
        i2c.write(bytes([registers[reg]]))

i2c.init(mode=I2C.SLAVE, addr=SLAVE_ADDR)
print(f"ESP32 I2C slave ready at 0x{SLAVE_ADDR:02X}")
print("I2C slave on GPIO21 (SDA) / GPIO22 (SCL)")
print("Register map: 0x00=temp_high, 0x01=temp_low, 0x02=config")

# Keep the slave alive
while True:
    pass