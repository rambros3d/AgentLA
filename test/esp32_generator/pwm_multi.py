"""PWM multi-range test - various frequencies and duty cycles
Wiring: ESP32 GPIO17 → RP2350 GPIO6
"""
from machine import Pin
import time

PIN = Pin(17, Pin.OUT, value=0)

tests = [
    # (frequency_hz, duty_percent, duration_ms)
    (500, 50, 200),    # 500Hz 50%
    (500, 20, 200),    # 500Hz 20%
    (500, 80, 200),    # 500Hz 80%
    (1000, 50, 200),   # 1kHz 50%
    (1000, 25, 200),   # 1kHz 25%
    (1000, 75, 200),   # 1kHz 75%
    (2000, 50, 200),   # 2kHz 50%
    (5000, 50, 200),   # 5kHz 50%
    (5000, 10, 200),   # 5kHz 10%
    (5000, 90, 200),   # 5kHz 90%
]

for freq, duty, dur_ms in tests:
    period_us = 1000000 // freq
    high_us = period_us * duty // 100
    low_us = period_us - high_us
    cycles = (dur_ms * freq) // 1000
    
    print(f"PWM {freq}Hz {duty}% ({dur_ms}ms)")
    for _ in range(cycles):
        PIN.value(1)
        time.sleep_us(high_us)
        PIN.value(0)
        time.sleep_us(low_us)

print("PWM MULTI Done")
