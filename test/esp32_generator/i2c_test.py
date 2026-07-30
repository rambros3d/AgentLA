"""I2C test - bit-banged, loops continuously for 5 seconds
Wiring: ESP32 GPIO17 (SDA) → RP2350 GPIO6 (SDA)
        ESP32 GPIO19 (SCL) → RP2350 GPIO7 (SCL)
Uses ESP32 internal pullups (~45kΩ).
"""
from machine import Pin
import time

sda = Pin(17, Pin.OUT, value=1)
scl = Pin(19, Pin.OUT, value=1)
sda_in = Pin(17, Pin.IN, Pin.PULL_UP)

I2C_DELAY = 5

def i2c_start():
    sda.value(1); scl.value(1); time.sleep_us(I2C_DELAY)
    sda.value(0); time.sleep_us(I2C_DELAY)
    scl.value(0); time.sleep_us(I2C_DELAY)

def i2c_stop():
    sda.value(0); scl.value(0); time.sleep_us(I2C_DELAY)
    scl.value(1); time.sleep_us(I2C_DELAY)
    sda.value(1); time.sleep_us(I2C_DELAY)

def i2c_write_bit(b):
    sda.value(b); time.sleep_us(I2C_DELAY)
    scl.value(1); time.sleep_us(I2C_DELAY)
    scl.value(0); time.sleep_us(I2C_DELAY)

def i2c_write_byte(byte):
    for i in range(7, -1, -1):
        i2c_write_bit((byte >> i) & 1)
    # ACK slot
    sda_in.init(Pin.IN, Pin.PULL_UP); time.sleep_us(I2C_DELAY)
    scl.value(1); time.sleep_us(I2C_DELAY)
    ack = not sda_in.value()
    scl.value(0); time.sleep_us(I2C_DELAY)
    sda.value(0); sda.init(Pin.OUT); time.sleep_us(I2C_DELAY)
    return ack

def i2c_read_byte(ack):
    byte = 0
    sda_in.init(Pin.IN, Pin.PULL_UP)
    for i in range(7, -1, -1):
        time.sleep_us(I2C_DELAY); scl.value(1); time.sleep_us(I2C_DELAY)
        if sda_in.value(): byte |= (1 << i)
        scl.value(0); time.sleep_us(I2C_DELAY)
    sda.value(0 if ack else 1); sda.init(Pin.OUT); time.sleep_us(I2C_DELAY)
    scl.value(1); time.sleep_us(I2C_DELAY)
    scl.value(0); time.sleep_us(I2C_DELAY)
    return byte

print("I2C LOOP: 5s of continuous transactions")

start = time.ticks_ms()
while time.ticks_diff(time.ticks_ms(), start) < 5000:
    # Transaction 1: Write 0x48 reg=0x00 data=0xAA
    i2c_start()
    i2c_write_byte(0x90)  # 0x48 << 1 | W
    i2c_write_byte(0x00)
    i2c_write_byte(0xAA)
    i2c_stop()
    time.sleep_ms(5)

    # Transaction 2: Read from 0x48 reg=0x01
    i2c_start()
    i2c_write_byte(0x90)
    i2c_write_byte(0x01)
    i2c_stop()
    time.sleep_ms(2)

    i2c_start()
    i2c_write_byte(0x91)  # 0x48 << 1 | R
    i2c_read_byte(False)
    i2c_stop()
    time.sleep_ms(5)

    # Transaction 3: Write 0x50 data=0xBB,0xCC
    i2c_start()
    i2c_write_byte(0xA0)  # 0x50 << 1 | W
    i2c_write_byte(0xBB)
    i2c_write_byte(0xCC)
    i2c_stop()
    time.sleep_ms(5)

sda.init(Pin.OUT); scl.init(Pin.OUT)
print("I2C Done")
