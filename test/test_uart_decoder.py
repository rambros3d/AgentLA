#!/usr/bin/env python3
"""
UART Decoder Test
Coordinates ESP32 signal generation with AgentLA UART decode capture.
"""
import sys
import serial
import time

BAUD = 115200
TIMEOUT = 10

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

def test_uart_decode(rp_port, esp_port):
    print("=" * 60)
    print("UART DECODER TEST")
    print("=" * 60)
    
    # Connect to both devices
    print(f"\nConnecting to AgentLA (RP2350) on {rp_port}...")
    rp_ser = serial.Serial(rp_port, BAUD, timeout=TIMEOUT)
    time.sleep(1)
    rp_ser.reset_input_buffer()
    time.sleep(0.5)
    
    print(f"Connecting to ESP32 on {esp_port}...")
    esp_ser = serial.Serial(esp_port, BAUD, timeout=TIMEOUT)
    time.sleep(1)
    esp_ser.reset_input_buffer()
    time.sleep(0.5)
    
    # Step 1: Configure AgentLA for UART decode
    print("\n--- Step 1: Configure AgentLA for UART decode ---")
    lines = send_cmd(rp_ser, "CONFIGURE channels=6 rate=1000000")
    for l in lines:
        print(f"  {l}")
    
    # Step 2: Start capture (will wait for data)
    print("\n--- Step 2: Start UART decode capture ---")
    # Send capture command but don't wait for DONE yet
    rp_ser.write(b"CAPTURE samples=4096 mode=DECODED protocol=UART rx=6 baud=115200\r\n")
    time.sleep(0.5)  # Give device time to start capturing
    
    # Step 3: Trigger ESP32 UART test
    print("\n--- Step 3: Trigger ESP32 UART test ---")
    esp_ser.write(b"u\r\n")
    time.sleep(0.5)
    
    # Read ESP32 response
    esp_lines = []
    deadline = time.time() + 3
    while time.time() < deadline:
        line = esp_ser.readline().decode().strip()
        if line:
            esp_lines.append(line)
            print(f"  ESP32: {line}")
    
    # Step 4: Wait for capture to complete and read results
    print("\n--- Step 4: Waiting for capture results ---")
    lines = []
    deadline = time.time() + TIMEOUT
    while time.time() < deadline:
        line = rp_ser.readline().decode().strip()
        if line:
            lines.append(line)
            print(f"  RP2350: {line}")
            if line.startswith("DONE") or line.startswith("ERR"):
                break
    
    # Analyze results
    print("\n--- Results ---")
    print(f"  Total lines received: {len(lines)}")
    
    has_uart = any("UART" in l for l in lines)
    has_data = any("DATA" in l or "RX" in l for l in lines)
    has_done = any(l.startswith("DONE") for l in lines)
    has_error = any(l.startswith("ERR") for l in lines)
    
    print(f"  UART decoded: {has_uart}")
    print(f"  Data found: {has_data}")
    print(f"  Capture complete: {has_done}")
    print(f"  Errors: {has_error}")
    
    ok = has_uart or has_data
    print(f"\n  {'PASS ✓' if ok else 'FAIL ✗'}")
    
    rp_ser.close()
    esp_ser.close()
    return ok

if __name__ == "__main__":
    rp_port = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
    esp_port = sys.argv[2] if len(sys.argv) > 2 else "/dev/ttyUSB0"
    test_uart_decode(rp_port, esp_port)
