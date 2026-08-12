#!/usr/bin/env python3
"""Integration test script for WCH CH32V203 2-wire RV-SWD debugging and flash programming."""

import serial
import time
import sys

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

def send_cmd(ser, cmd, wait=0.3, timeout=2.0):
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    time.sleep(wait)
    lines = []
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors='replace').strip()
            if line:
                lines.append(line)
        else:
            time.sleep(0.05)
    return lines

def main():
    print("=== CH32V203 2-Wire RV-SWD Test ===")
    try:
        ser = serial.Serial(PORT, BAUD, timeout=2, rtscts=False, dsrdtr=False)
        ser.dtr = True
        ser.rts = True
    except Exception as e:
        print(f"Error opening port {PORT}: {e}")
        return False

    time.sleep(1)

    print("\n1. Identify Device...")
    lines = send_cmd(ser, 'IDENTIFY', wait=0.3)
    for l in lines: print(f"  {l}")

    print("\n2. Probe CH32V203 Target (2-Wire RV-SWD)...")
    lines = send_cmd(ser, 'CH32 DETECT mode=2wire', wait=0.5)
    for l in lines: print(f"  {l}")

    print("\n3. Erase Page at 0x08000000...")
    lines = send_cmd(ser, 'CH32 ERASE 0x08000000 128', wait=0.8)
    for l in lines: print(f"  {l}")

    print("\n4. Program Test Pattern at 0x08000000...")
    test_hex = "87654321" * 8  # 32 bytes
    lines = send_cmd(ser, f'CH32 FLASH 0x08000000 {test_hex}', wait=1.0)
    for l in lines: print(f"  {l}")

    ser.close()
    print("\n=== CH32V203 Test Completed ===")
    return True

if __name__ == "__main__":
    main()
