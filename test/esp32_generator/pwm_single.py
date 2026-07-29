"""PWM test using ESP32 HARDWARE PWM peripheral
Wiring: ESP32 GPIO17 → RP2350 GPIO6
Reads freq/duty from files on ESP32 filesystem.
"""
from machine import Pin, PWM
import time

# Read freq/duty from files with validation
freq = 1000
duty = 50

try:
    with open("/freq.txt") as f:
        freq = int(f.read().strip())
except (ValueError, OSError):
    pass

try:
    with open("/duty.txt") as f:
        duty = int(f.read().strip())
except (ValueError, OSError):
    pass

# Validate ranges
freq = max(1, min(freq, 40000000))
duty = max(0, min(duty, 100))

# Hardware PWM on GPIO17
pwm = PWM(Pin(17))
try:
    pwm.freq(freq)
    pwm.duty_u16(int(65535 * duty / 100 + 0.5))
    print(f"PWM HW: {freq}Hz {duty}%")
    time.sleep_ms(200)
finally:
    pwm.deinit()
    Pin(17).init(Pin.IN)
print("DONE")
