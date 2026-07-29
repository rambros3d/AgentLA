"""PWM sweep test - outputs each frequency/duty for exactly 100ms with markers
Wiring: ESP32 GPIO17 → RP2350 GPIO6
Usage: mpremote connect /dev/ttyUSB0 run pwm_sweep.py
"""
from machine import Pin
import time

PIN = Pin(17, Pin.OUT, value=0)

tests = [
    # (frequency_hz, duty_percent)
    (500, 50),     # baseline
    (500, 20),     # low duty
    (500, 80),     # high duty
    (1000, 50),    # 1kHz
    (1000, 25),    # 1kHz low
    (1000, 75),    # 1kHz high
    (2000, 50),    # 2kHz
    (2000, 10),    # 2kHz extreme low
    (2000, 90),    # 2kHz extreme high
    (5000, 50),    # 5kHz
    (5000, 30),    # 5kHz
    (5000, 70),    # 5kHz
]

DURATION_MS = 100  # each test runs 100ms

for freq, duty in tests:
    period_us = 1000000 // freq
    high_us = period_us * duty // 100
    low_us = period_us - high_us
    cycles = (DURATION_MS * freq) // 1000
    
    print(f"TEST {freq}Hz {duty}%")
    for _ in range(cycles):
        PIN.value(1)
        time.sleep_us(high_us)
        PIN.value(0)
        time.sleep_us(low_us)

PIN.init(Pin.IN)
print("DONE")
