"""SPI-only test - runs once and exits"""
from machine import Pin
import time

PIN_MOSI = Pin(19, Pin.OUT, value=1)
PIN_SCLK = Pin(18, Pin.OUT, value=0)
PIN_MISO = Pin(17, Pin.IN)
PIN_CS = Pin(16, Pin.OUT, value=1)

def spi_transfer(tx):
    PIN_CS.value(0)
    time.sleep_us(1)
    rx = []
    for byte in tx:
        received = 0
        for b in range(7, -1, -1):
            PIN_MOSI.value((byte >> b) & 1)
            time.sleep_us(1)
            PIN_SCLK.value(1)
            time.sleep_us(1)
            if PIN_MISO.value():
                received |= (1 << b)
            PIN_SCLK.value(0)
            time.sleep_us(1)
        rx.append(received)
    time.sleep_us(1)
    PIN_CS.value(1)
    return rx

print("SPI: Sending [0xAA, 0xBB, 0xCC] CPOL=0 CPHA=0")
PIN_CS.value(1)
PIN_SCLK.value(0)

rx = spi_transfer([0xAA, 0xBB, 0xCC])
print(f"  MISO=[0x{rx[0]:02X},0x{rx[1]:02X},0x{rx[2]:02X}]")
print("SPI Done")
