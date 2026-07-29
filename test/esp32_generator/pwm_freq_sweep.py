"""PWM frequency sweep: 100Hz to 40kHz at 50% duty
Wiring: ESP32 GPIO17 → RP2350 GPIO6
Each frequency runs for 100ms.
"""
from machine import Pin
import time

PIN = Pin(17, Pin.OUT, value=0)

# Frequencies to test: 100Hz → 40kHz
freqs = [
    100, 200, 500,
    1000, 2000, 5000,
    10000, 15000, 20000,
    25000, 30000, 40000,
]

DUTY = 50
DURATION_MS = 100

for freq in freqs:
    period_us = 1000000 // freq
    high_us = period_us * DUTY // 100
    low_us = period_us - high_us
    cycles = (DURATION_MS * freq) // 1000
    
    print(f"TEST {freq}Hz {DUTY}%")
    for _ in range(cycles):
        PIN.value(1)
        time.sleep_us(high_us)
        PIN.value(0)
        time.sleep_us(low_us)

PIN.init(Pin.IN)
print("DONE")
