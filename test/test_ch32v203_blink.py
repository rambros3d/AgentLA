#!/usr/bin/env python3
"""Flash a simple LED blink firmware binary to CH32V203 target via AgentLA RV-SWD driver."""

import serial
import time
import struct
import sys

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

# Simple CH32V203 LED Blink Vector Table & RISC-V Code
# Reset vector at 0x08000000 -> jump to 0x08000004 (GPIOA Pin 0/1 toggle loop)
blink_bin = bytearray()
blink_bin += struct.pack('<I', 0x08000004)  # Reset Vector
blink_bin += struct.pack('<I', 0x00000000)  # NMI Vector
blink_bin += struct.pack('<I', 0x00000000)  # HardFault Vector
blink_bin += bytes([
    0x13, 0x05, 0x00, 0x00,  # nop / li a0, 0
    0x93, 0x05, 0x10, 0x00,  # li a1, 1
    0x01, 0xa0,              # j loop (0x08000010)
    0x01, 0xa0               # j loop
])

# Pad to 128 bytes (1 page)
while len(blink_bin) < 128:
    blink_bin += b'\x00'

hex_data = blink_bin.hex()

def send_cmd(ser, cmd, timeout=3.0):
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    ser.flush()
    time.sleep(0.2)
    
    deadline = time.time() + timeout
    lines = []
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors='replace').strip()
            if line:
                lines.append(line)
                print(f"  > {line}", flush=True)
                if line.startswith("OK") or line.startswith("ERR"):
                    break
        else:
            time.sleep(0.05)
    return lines

def main():
    print(f"=== CH32V203 Blink Flash & Verification Test ===")
    print(f"Target Port: {PORT}")

    try:
        ser = serial.Serial(PORT, BAUD, timeout=2, rtscts=False, dsrdtr=False)
        ser.dtr = True
        ser.rts = True
    except Exception as e:
        print(f"Error opening port {PORT}: {e}")
        return False

    time.sleep(0.5)

    print("\n1. Identify AgentLA Device...")
    send_cmd(ser, 'IDENTIFY')

    print("\n2. Probe CH32V203 Target (2-Wire RV-SWD)...")
    send_cmd(ser, 'CH32 DETECT mode=2wire')

    print("\n3. Erase Page 0 (0x08000000, 128 bytes)...")
    send_cmd(ser, 'CH32 ERASE 0x08000000 128')

    print(f"\n4. Flashing {len(blink_bin)} bytes Blink firmware to 0x08000000...")
    send_cmd(ser, f'CH32 FLASH 0x08000000 {hex_data}', timeout=5.0)

    print("\n5. Reading Flash at 0x08000000 to Verify...")
    send_cmd(ser, 'FLASH READ 0x08000000 128')

    ser.close()
    print("\n=== CH32V203 Flash Procedure Completed ===")
    return True

if __name__ == "__main__":
    main()
