"""UART-only test - runs once and exits
Correct wiring: ESP32 → RP2350
  GPIO17 → GPIO6 (TX/MOSI)
"""
from machine import Pin
import time

PIN_TX = Pin(17, Pin.OUT, value=1)

def uart_send_byte(byte):
    bit_time = 1000000 // 115200  # ~8 us
    PIN_TX.value(0)
    time.sleep_us(bit_time)
    for i in range(8):
        PIN_TX.value((byte >> i) & 1)
        time.sleep_us(bit_time)
    PIN_TX.value(1)
    time.sleep_us(bit_time)

print("UART: Sending 'HELLO' at 115200")
PIN_TX.init(Pin.OUT)
PIN_TX.value(1)

for ch in "HELLO":
    uart_send_byte(ord(ch))
    print(f"  Sent '{ch}'")

print("UART Done")
