#!/usr/bin/env python3
"""Hardware verification for programmer interface on RP2350 + Blue Pill."""
import serial
import time
import sys
import subprocess

BAUD = 115200
RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
TIMEOUT = 5

def send_cmd(ser, cmd, wait=0.3):
    ser.write((cmd + "\r\n").encode())
    time.sleep(wait)
    lines = []
    while ser.in_waiting:
        line = ser.readline().decode(errors="replace").strip()
        if line:
            lines.append(line)
    return lines

def wait_boot(ser):
    for i in range(20):
        line = ser.readline()
        if b">" in line or b"AGENTLA" in line or b"OK" in line:
            return True
        time.sleep(0.1)
    return False

def flash():
    result = subprocess.run(
        ["picotool", "load", "build/AgentLA.uf2", "-t", "uf2"],
        capture_output=True, text=True
    )
    print(result.stdout)
    if result.returncode != 0:
        print("Flash failed:", result.stderr)
        return False
    print("Flashed successfully")
    return True

def main():
    # Step 1: Flash
    print("=== Step 1: Flash firmware ===")
    print("Hold BOOTSEL on RP2350, connect USB, then press Enter")
    input()
    if not flash():
        return

    # Step 2: Connect serial
    print("\n=== Step 2: Connect serial monitor ===")
    print(f"Opening {RP_PORT} at {BAUD} baud...")
    ser = serial.Serial(RP_PORT, BAUD, timeout=TIMEOUT)
    time.sleep(1)
    ser.reset_input_buffer()

    # Wait for boot messages
    print("Waiting for boot...")
    time.sleep(2)
    while ser.in_waiting:
        print(ser.readline().decode(errors="replace").strip())

    # Step 3: Identify
    print("\n=== Step 3: IDENTIFY ===")
    ser.write(b"IDENTIFY\r\n")
    time.sleep(0.5)
    while ser.in_waiting:
        print(ser.readline().decode(errors="replace").strip())

    # Step 4: PROG PINS
    print("\n=== Step 4: PROG PINS ===")
    for line in send_cmd(ser, "PROG PINS"):
        print(line)

    # Step 5: Enter programmer mode
    print("\n=== Step 5: Enter programmer mode ===")
    for line in send_cmd(ser, "MODE PROG ON"):
        print(line)

    # Step 6: Mode status
    print("\n=== Step 6: PROG MODE STATUS ===")
    for line in send_cmd(ser, "MODE PROG STATUS"):
        print(line)

    # Step 7: Power on
    print("\n=== Step 7: PROG POWER ON ===")
    for line in send_cmd(ser, "PROG POWER ON"):
        print(line)

    # Step 8: Power status
    print("\n=== Step 8: PROG POWER STATUS ===")
    for line in send_cmd(ser, "PROG POWER STATUS"):
        print(line)

    # Step 9: nRESET pulse
    print("\n=== Step 9: PROG NRESET PULSE ===")
    for line in send_cmd(ser, "PROG NRESET PULSE"):
        print(line)

    # Step 10: Exit programmer mode
    print("\n=== Step 10: MODE PROG OFF ===")
    for line in send_cmd(ser, "MODE PROG OFF"):
        print(line)

    # Step 11: STM32 SYNC test (require Blue Pill with BOOT0=1, nRESET pulsed)
    print("\n=== Step 11: PROG STM32 SYNC ===")
    print("Ensure Blue Pill BOOT0=1 and nRESET pulsed, then press Enter")
    input()
    for line in send_cmd(ser, "MODE PROG ON"):
        print(line)
    for line in send_cmd(ser, "PROG STM32 SYNC"):
        print(line)

    # Step 12: UART loopback (short GPIO8 to GPIO9 on RP2350 side)
    print("\n=== Step 12: PROG UART TEST ===")
    for line in send_cmd(ser, "PROG UART TEST"):
        print(line)

    ser.close()
    print("\n=== Verification complete ===")

if __name__ == "__main__":
    main()
