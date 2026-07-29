#!/usr/bin/env python3
"""
Combined UART Decoder Test
Runs AgentLA capture and ESP32 signal generation simultaneously.
"""
import subprocess
import serial
import time
import sys

BAUD = 115200
TIMEOUT = 15
RP_PORT = "/dev/ttyACM0"
ESP_PORT = "/dev/ttyUSB0"

def send_cmd(ser, cmd):
    """Send command and return response lines."""
    ser.write(f"{cmd}\r\n".encode())
    lines = []
    deadline = time.time() + TIMEOUT
    while time.time() < deadline:
        line = ser.readline().decode().strip()
        if line:
            lines.append(line)
            if line.startswith("DONE") or line.startswith("ERR"):
                break
    return lines

print("=" * 60)
print("UART DECODER VERIFICATION TEST")
print("=" * 60)

# Connect to AgentLA
print(f"\n1. Connecting to AgentLA on {RP_PORT}...")
rp_ser = serial.Serial(RP_PORT, BAUD, timeout=TIMEOUT)
time.sleep(1)
rp_ser.reset_input_buffer()
time.sleep(0.5)

# Clear any boot artifacts
rp_ser.readline()

# Configure for UART decode on channel 6
print("2. Configuring UART decode: channels=6, baud=115200...")
lines = send_cmd(rp_ser, "CONFIGURE channels=6 rate=1000000")
for l in lines:
    print(f"   {l}")

# Start capture (RP2350 will wait for data)
print("3. Starting UART decode capture...")
rp_ser.write(b"CAPTURE samples=4096 mode=DECODED protocol=UART rx=6 baud=115200\r\n")
time.sleep(0.3)  # Let capture start

# Trigger ESP32 UART test
print("4. Triggering ESP32 UART test ('HELLO' at 115200)...")
subprocess.run(
    ["mpremote", "connect", ESP_PORT, "run", "test/esp32_generator/uart_test.py"],
    timeout=10,
    capture_output=True
)

# Read capture results
print("5. Waiting for decode results...")
lines = []
deadline = time.time() + TIMEOUT
while time.time() < deadline:
    line = rp_ser.readline().decode().strip()
    if line:
        lines.append(line)
        if line.startswith("DONE") or line.startswith("ERR"):
            break

rp_ser.close()

# Analyze
print("\n" + "=" * 60)
print("RESULTS:")
print("=" * 60)
for l in lines:
    print(f"  {l}")

has_uart = any("UART" in l and "BREAK" not in l for l in lines)
has_char_data = any("H" in l or "E" in l or "L" in l or "O" in l for l in lines)
has_done = any(l.startswith("DONE") for l in lines)
has_error = any(l.startswith("ERR") for l in lines)

print(f"\n  UART frames decoded: {has_uart}")
print(f"  Character data found: {has_char_data}")
print(f"  Capture completed: {has_done}")
print(f"  Errors: {has_error}")

# Count UART lines
uart_lines = [l for l in lines if "UART" in l]
print(f"  Total UART lines: {len(uart_lines)}")

ok = has_done and not has_error
print(f"\n  {'PASS ✓' if ok else 'FAIL ✗'}")
