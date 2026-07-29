"""SPI 8MHz loop - sends [0xAA,0xBB,0xCC] at 8MHz
Wiring: ESP32 GPIO17→RP2350 GPIO6 (MOSI), GPIO19→GPIO7 (SCLK),
        GPIO26→GPIO27 (MISO), GPIO25→GPIO26 (CS)
"""
from machine import SPI, Pin
import time

spi = SPI(1, baudrate=8000000, polarity=0, phase=0,
          sck=Pin(19), mosi=Pin(17), miso=Pin(26))
cs = Pin(25, Pin.OUT, value=1)

print("SPI LOOP: 8MHz")
start = time.ticks_ms()
while time.ticks_diff(time.ticks_ms(), start) < 3000:
    cs.value(0)
    spi.write(bytes([0xAA, 0xBB, 0xCC]))
    cs.value(1)

print("Done")
spi.deinit()
