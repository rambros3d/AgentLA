"""
AgentLA ESP32 Test Signal Generator (MicroPython)
Pin mapping (ESP32 → RP2350):
  GPIO17 → RP2350 GPIO6 (SDA / MOSI / TX / PWM)
  GPIO19 → RP2350 GPIO7 (SCL / SCLK)
  GPIO26 → RP2350 GPIO27 (MISO)
  GPIO25 → RP2350 GPIO26 (CS)
  GPIO2  → RP2350 RUN (reset)
"""

from machine import SPI, Pin
import time

# Direct GPIO for I2C/UART/PWM
PIN_SDA = Pin(17, Pin.OUT, value=1)   # GPIO17 → RP2350 GPIO6
PIN_SCL = Pin(19, Pin.OUT, value=1)   # GPIO19 → RP2350 GPIO7
PIN_CS = Pin(25, Pin.OUT, value=1)    # GPIO25 → RP2350 GPIO26
PIN_RUN = Pin(2, Pin.OUT, value=1)

I2C_DELAY = 5  # microseconds

# ─── I2C ─────────────────────────────────────────────────────────────────

def i2c_start():
    PIN_SDA.value(1)
    PIN_SCL.value(1)
    time.sleep_us(I2C_DELAY)
    PIN_SDA.value(0)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(0)
    time.sleep_us(I2C_DELAY)

def i2c_stop():
    PIN_SDA.value(0)
    PIN_SCL.value(0)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(1)
    time.sleep_us(I2C_DELAY)
    PIN_SDA.value(1)
    time.sleep_us(I2C_DELAY)

def i2c_write_bit(b):
    PIN_SDA.value(b)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(1)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(0)
    time.sleep_us(I2C_DELAY)

def i2c_write_byte(byte):
    for i in range(7, -1, -1):
        i2c_write_bit((byte >> i) & 1)
    # ACK
    PIN_SDA.init(Pin.IN)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(1)
    time.sleep_us(I2C_DELAY)
    ack = not PIN_SDA.value()
    PIN_SCL.value(0)
    time.sleep_us(I2C_DELAY)
    PIN_SDA.init(Pin.OUT)
    return ack

def i2c_read_byte(ack):
    byte = 0
    PIN_SDA.init(Pin.IN)
    for i in range(7, -1, -1):
        time.sleep_us(I2C_DELAY)
        PIN_SCL.value(1)
        time.sleep_us(I2C_DELAY)
        if PIN_SDA.value():
            byte |= (1 << i)
        PIN_SCL.value(0)
        time.sleep_us(I2C_DELAY)
    PIN_SDA.init(Pin.OUT)
    PIN_SDA.value(0 if ack else 1)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(1)
    time.sleep_us(I2C_DELAY)
    PIN_SCL.value(0)
    time.sleep_us(I2C_DELAY)
    return byte

def test_i2c():
    print("I2C: addr=0x48 W [0x00,0x7F] R [0xAA]")
    PIN_SDA.init(Pin.OUT)
    PIN_SCL.init(Pin.OUT)
    PIN_SDA.value(1)
    PIN_SCL.value(1)

    i2c_start()
    i2c_write_byte(0x90)  # 0x48 << 1 | W
    i2c_write_byte(0x00)
    i2c_write_byte(0x7F)
    i2c_stop()

    time.sleep_us(100)

    i2c_start()
    i2c_write_byte(0x91)  # 0x48 << 1 | R
    val = i2c_read_byte(False)
    print(f"  Read=0x{val:02X}")
    i2c_stop()

    time.sleep_us(100)
    print("  I2C Done")

# ─── SPI ─────────────────────────────────────────────────────────────────

# Hardware SPI: SPI(1)=HSPI, explicit pins match wiring
_spi = SPI(1, baudrate=1000000, polarity=0, phase=0,
           sck=Pin(19), mosi=Pin(17), miso=Pin(26))

def spi_transfer(tx):
    PIN_CS.value(0)
    _spi.write(bytes(tx))
    PIN_CS.value(1)

def test_spi():
    print("SPI (HW): MOSI=[0xAA,0xBB,0xCC] CPOL=0 CPHA=0")
    PIN_CS.value(1)
    spi_transfer([0xAA, 0xBB, 0xCC])
    print("  SPI Done")

# ─── UART ────────────────────────────────────────────────────────────────

def uart_send_byte(byte):
    bit_time = 1000000 // 115200  # ~8 us
    PIN_SDA.value(0)
    time.sleep_us(bit_time)
    for i in range(8):
        PIN_SDA.value((byte >> i) & 1)
        time.sleep_us(bit_time)
    PIN_SDA.value(1)
    time.sleep_us(bit_time)

def test_uart():
    print("UART: 'HELLO' at 115200")
    PIN_SDA.init(Pin.OUT)
    PIN_SDA.value(1)

    for ch in "HELLO":
        uart_send_byte(ord(ch))
    print("  UART Done")

# ─── PWM ─────────────────────────────────────────────────────────────────

def test_pwm():
    print("PWM: 1 kHz, 60% duty on GPIO19")
    PIN_SDA.init(Pin.OUT)

    for _ in range(10):
        PIN_SDA.value(1)
        time.sleep_us(600)
        PIN_SDA.value(0)
        time.sleep_us(400)
    print("  PWM Done")

# ─── RESET ───────────────────────────────────────────────────────────────

def reset_rp2350():
    print("Resetting RP2350...")
    PIN_RUN.value(0)
    time.sleep_ms(100)
    PIN_RUN.value(1)
    time.sleep_ms(500)
    print("  Reset complete")

# ─── MAIN ────────────────────────────────────────────────────────────────

print("AgentLA ESP32 Test Generator")
print("Commands: i=I2C s=SPI u=UART p=PWM a=ALL r=RESET h=HELP")

import sys

def help():
    print("Commands:")
    print("  i - I2C test")
    print("  s - SPI test")
    print("  u - UART test")
    print("  p - PWM test")
    print("  a - All tests")
    print("  r - Reset RP2350")

while True:
    try:
        cmd = input().strip()
        if cmd == 'i': test_i2c()
        elif cmd == 's': test_spi()
        elif cmd == 'u': test_uart()
        elif cmd == 'p': test_pwm()
        elif cmd == 'a':
            test_i2c()
            time.sleep(0.5)
            test_spi()
            time.sleep(0.5)
            test_uart()
            time.sleep(0.5)
            test_pwm()
        elif cmd == 'r': reset_rp2350()
        elif cmd == 'h': help()
    except EOFError:
        break
    except KeyboardInterrupt:
        break
