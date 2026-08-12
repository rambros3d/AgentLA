#!/usr/bin/env python3
"""Automated Blue Pill STM32F103 flash via RP2350 UART bootloader bridge."""
import serial, time, struct, sys

RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

bin_data = bytearray()
bin_data += struct.pack('<I', 0x20005000)
bin_data += struct.pack('<I', 0x08000011)
bin_data += struct.pack('<I', 0x08000011)
bin_data += struct.pack('<I', 0x08000011)
bin_data += bytes([0xFE, 0xE7])
while len(bin_data) < 32:
    bin_data += b'\x00'
hex_data = bin_data.hex()

def drain(ser, timeout=1.0):
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            if ser.in_waiting:
                ser.read(ser.in_waiting)
        except: break
        time.sleep(0.05)

def send_cmd(ser, cmd, wait=0.5):
    drain(ser, 0.1)
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    time.sleep(wait)
    lines = []
    try:
        while ser.in_waiting:
            l = ser.readline()
            if l: lines.append(l.decode(errors='replace').strip())
    except: pass
    return lines

def main():
    print("=== Blue Pill Flash Script ===")
    ser = serial.Serial(RP_PORT, BAUD, timeout=2)
    time.sleep(1)
    drain(ser, 2.0)

    # Enter programmer mode (retry a couple of times in case of boot noise)
    print("\n1. Enter programmer mode...")
    for retry in range(3):
        lines = send_cmd(ser, 'MODE PROG ON', wait=0.4)
        for l in lines: print(f"  {l}")
        if any('OK PROG MODE ON' in l for l in lines):
            break
    else:
        print("  Failed to enter programmer mode after 3 tries")
        ser.close()
        return

    print("\n2. Power on Blue Pill...")
    for l in send_cmd(ser, 'PROG POWER ON', wait=0.3): print(f"  {l}")

    print("\n3. Pulse nRST to trigger UART bootloader")
    print("   (ensure Blue Pill BOOT0=HIGH/3.3V before this step)")
    for l in send_cmd(ser, 'PROG NRESET PULSE', wait=0.3): print(f"  {l}")
    time.sleep(0.3)

    print("\n4. SYNC with STM32 UART bootloader...")
    synced = False
    for attempt in range(3):
        lines = send_cmd(ser, 'PROG STM32 SYNC', wait=1.5)
        for l in lines: print(f"  {l}")
        if any('OK STM32 SYNCED' in l for l in lines):
            synced = True
            break
    if not synced:
        print("\n  SYNC FAILED! Blue Pill BOOT0 must be HIGH (3.3V) before nRST pulse.")
        print("  Connect BOOT0 → 3.3V, then rerun this script.")
        ser.close()
        return

    print(f"\n5. Writing {len(bin_data)} bytes...")
    cmd = f"PROG STM32 WRITE 0x08000000 {hex_data}"
    for l in send_cmd(ser, cmd, wait=2.5): print(f"  {l}")

    print("\n6. GO...")
    for l in send_cmd(ser, 'PROG STM32 GO', wait=1.0): print(f"  {l}")

    print("\n7. Exit programmer mode...")
    for l in send_cmd(ser, 'MODE PROG OFF', wait=0.3): print(f"  {l}")

    ser.close()
    print("\n=== Flash complete! ===")
    print("Disconnect BOOT0 from 3.3V and power-cycle Blue Pill to run the blink app.")

if __name__ == "__main__":
    main()
