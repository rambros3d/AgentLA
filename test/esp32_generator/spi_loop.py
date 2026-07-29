"""SPI loop test - configurable baudrate via /baudrate.txt on ESP32
Correct wiring: ESP32 → RP2350
  GPIO17 → GPIO6 (MOSI)
  GPIO19 → GPIO7 (SCLK)
  GPIO26 → GPIO27 (MISO)
  GPIO25 → GPIO26 (CS)
"""
from machine import SPI, Pin
import time

# Read baudrate from file or default to 1MHz
baudrate = 1000000
try:
    with open("/baudrate.txt") as f:
        baudrate = int(f.read().strip())
except:
    pass

# Hardware SPI with explicit pin mapping
spi = SPI(1, baudrate=baudrate, polarity=0, phase=0,
          sck=Pin(19), mosi=Pin(17), miso=Pin(26))
cs = Pin(25, Pin.OUT, value=1)

def spi_transfer(tx):
    """Send bytes via hardware SPI"""
    cs.value(0)
    spi.write(bytes(tx))
    cs.value(1)

print(f"SPI LOOP: {baudrate//1000000}MHz")
cs.value(1)

# Run for 3 seconds so RP2350 can capture it
start = time.ticks_ms()
while time.ticks_diff(time.ticks_ms(), start) < 3000:
    spi_transfer([0xAA, 0xBB, 0xCC])

print("SPI LOOP Done")
spi.deinit()
