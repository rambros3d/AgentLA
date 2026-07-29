"""ADC test - generates analog signals using ESP32 PWM on GPIO26
Wiring: ESP32 GPIO26 -> RP2350 GPIO27 (ADC1)
Generates: PWM at various duty cycles (simulates analog voltages)
Also outputs DC via DAC on GPIO25 if available
"""
from machine import Pin, PWM
import time

# PWM on GPIO26 -> RP2350 GPIO27 (ADC1)
# PWM avg voltage = duty * 3.3V / 100
pwm = PWM(Pin(26))
pwm.freq(10000)  # 10kHz PWM (fast enough to look like DC to ADC)

print("ADC PWM TEST: duty cycle sweep (GPIO26->GPIO27)")

# Test 1: DC levels via PWM duty cycle (1s each)
duties = [(0, "0V"), (50, "1.65V"), (100, "3.3V")]
for duty, label in duties:
    print(f"  PWM {duty}% ({label})")
    pwm.duty_u16(int(65535 * duty / 100))
    time.sleep_ms(1000)

# Test 2: Ramp duty cycle (500us/step)
print("  Ramp 0->100->0%")
for d in range(0, 101, 5):
    pwm.duty_u16(int(65535 * d / 100))
    time.sleep_us(500)
for d in range(100, -1, -5):
    pwm.duty_u16(int(65535 * d / 100))
    time.sleep_us(500)

# Test 3: Various fixed duty cycles
print("  Duty sweep: 10,25,50,75,90%")
for d in [10, 25, 50, 75, 90]:
    pwm.duty_u16(int(65535 * d / 100))
    time.sleep_ms(500)

pwm.deinit()
Pin(26, Pin.IN)
print("ADC PWM Done")
