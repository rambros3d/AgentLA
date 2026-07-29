#!/usr/bin/env python3
"""Simple UART decoder test - starts capture, triggers ESP32, reads results."""
import serial
import time
import subprocess
import sys

BAUD = 115200
RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
ESP_PORT = "/dev/ttyUSB0"

print(f"Connecting to {RP_PORT}...")
rp = serial.Serial(RP_PORT, BAUD, timeout=5)
time.sleep(1)
rp.reset_input_buffer()
time.sleep(0.5)
rp.readline()  # flush boot

# Configure and start capture
print("CONFIGURE channels=6 rate=1000000")
rp.write(b"CONFIGURE channels=6 rate=1000000\r\n")
time.sleep(0.5)
while rp.in_waiting:
    print(f"  > {rp.readline().decode().strip()}")

# Start capture FIRST - device will wait for data
# Start ESP32 in background FIRST - it will send data immediately
print("Starting ESP32 UART test in background...")
esp_proc = subprocess.Popen(
    ["mpremote", "connect", ESP_PORT, "run", "test/esp32_generator/uart_test.py"],
    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
)
time.sleep(0.3)  # Let ESP32 start sending (subprocess.Popen is non-blocking)

# Now start capture - ESP32 data should be flowing
# Max buffer = 393216 samples (128K * 3), use 200000 for safety
print("CAPTURE samples=200000 mode=DECODED protocol=UART rx=6 baud=115200")
rp.write(b"CAPTURE samples=200000 mode=DECODED protocol=UART rx=6 baud=115200\r\n")

# Wait for ESP32 to finish
try:
    esp_stdout, _ = esp_proc.communicate(timeout=5)
    print(f"  ESP32 stdout: {esp_stdout.strip()}")
except subprocess.TimeoutExpired:
    esp_proc.kill()
    print("  ESP32 timed out")

# Read results
print("\nCapture results:")
lines = []
deadline = time.time() + 10
while time.time() < deadline:
    line = rp.readline().decode().strip()
    if line:
        lines.append(line)
        print(f"  {line}")
        if line.startswith("DONE") or line.startswith("ERR"):
            break

rp.close()

# Summary
uart_lines = [l for l in lines if "UART" in l]
print(f"\nTotal UART lines: {len(uart_lines)}")
print(f"Capture done: {any(l.startswith('DONE') for l in lines)}")
print(f"Errors: {any(l.startswith('ERR') for l in lines)}")
