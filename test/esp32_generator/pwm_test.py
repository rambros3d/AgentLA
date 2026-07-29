"""PWM test - generates 1kHz 60% duty cycle on GPIO17
Wiring: ESP32 GPIO17 → RP2350 GPIO6
"""
from machine import Pin
import time

PIN = Pin(17, Pin.OUT, value=0)

print("PWM TEST: 1kHz 60% duty on GPIO17")
# 1kHz = 1000us period, 60% = 600us high, 40% = 400us low
for _ in range(500):  # ~500ms of PWM
    PIN.value(1)
    time.sleep_us(600)
    PIN.value(0)
    time.sleep_us(400)

print("PWM Done")
