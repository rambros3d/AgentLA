"""SPI multi-rate test - verifies decoder at 1MHz, 2MHz, 4MHz
Correct wiring: ESP32 → RP2350
  GPIO17 → GPIO6 (MOSI)
  GPIO19 → GPIO7 (SCLK)
  GPIO26 → GPIO27 (MISO)
  GPIO25 → GPIO26 (CS)
"""
from machine import SPI, Pin
import time

# Hardware SPI with explicit pin mapping
cs = Pin(25, Pin.OUT, value=1)

def test_rate(baudrate, duration_ms=1000):
    """Test SPI at given baudrate for duration_ms"""
    spi = SPI(1, baudrate=baudrate, polarity=0, phase=0,
              sck=Pin(19), mosi=Pin(17), miso=Pin(26))
    
    data = bytes([0xAA, 0xBB, 0xCC])
    count = 0
    start = time.ticks_ms()
    while time.ticks_diff(time.ticks_ms(), start) < duration_ms:
        cs.value(0)
        spi.write(data)
        cs.value(1)
        count += 1
    
    spi.deinit()
    return count

print("SPI MULTI-RATE TEST")
print("Sending [0xAA, 0xBB, 0xCC] at various rates")

rates = [1000000, 2000000, 4000000]
for rate in rates:
    count = test_rate(rate, 500)
    print(f"  {rate//1000000}MHz: {count} transfers in 500ms")

print("DONE - start RP2350 capture with matching CONFIGURE rate")
