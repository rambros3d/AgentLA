#!/usr/bin/env python3
"""
Flash BluePill USB CDC Serial Firmware via AgentLA SWD
"""

import glob
import sys
import time
from pathlib import Path
import serial

FIRMWARE_BIN = Path(__file__).resolve().parent / "bluepill_usb" / ".pio" / "build" / "bluepill_f103c8" / "firmware.bin"
FLASH_BASE = 0x08000000
WRITE_CHUNK = 256

def find_agentla():
    candidates = sorted(glob.glob('/dev/ttyACM*') + glob.glob('/dev/ttyUSB*'))
    for port in candidates:
        try:
            ser = serial.Serial(port, 115200, timeout=1)
            time.sleep(0.1)
            ser.reset_input_buffer()
            ser.write(b'IDENTIFY\r\n')
            time.sleep(0.3)
            resp = ser.read(ser.in_waiting or 1024).decode(errors='replace')
            ser.close()
            if 'AGENTLA' in resp:
                return port
        except Exception:
            continue
    return None

def send_cmd(ser, cmd, timeout=3.0):
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    time.sleep(0.02)
    deadline = time.time() + timeout
    lines = []
    while time.time() < deadline:
        if ser.in_waiting:
            raw = ser.read(ser.in_waiting).decode(errors='replace')
            for ln in raw.splitlines():
                ln = ln.strip()
                if ln and not ln.startswith('> '):
                    lines.append(ln)
            if any(l.startswith('OK ') or l.startswith('ERR ') for l in lines):
                break
        time.sleep(0.01)
    return lines

def main():
    if not FIRMWARE_BIN.exists():
        print(f"Error: {FIRMWARE_BIN} not found!")
        sys.exit(1)

    fw = FIRMWARE_BIN.read_bytes()
    print(f"Loaded USB CDC Firmware: {len(fw)} bytes ({len(fw)/1024:.1f} KB)")

    port = find_agentla()
    if not port:
        print("Error: AgentLA probe not found on serial ports!")
        sys.exit(1)
    print(f"Connecting to AgentLA on {port}...")

    ser = serial.Serial(port, 115200, timeout=1)
    time.sleep(0.5)

    # 1. Check SWD target detection
    lines = send_cmd(ser, 'STM32 TEST')
    print("SWD Test:", lines)
    if not any('OK STM32 TEST PASS' in l or 'OK SWD TEST PASS' in l for l in lines):
        print("Failed to detect Blue Pill target over SWD!")
        sys.exit(1)

    # 2. Erase Flash
    erase_size = (len(fw) + 0x3FF) & ~0x3FF
    print(f"Erasing 0x{erase_size:X} bytes at 0x{FLASH_BASE:08X}...")
    lines = send_cmd(ser, f'FLASH ERASE 0x{FLASH_BASE:08X} 0x{erase_size:X}', timeout=5.0)
    print("Erase response:", lines)

    # 3. Write Flash
    chunks = [fw[i:i + WRITE_CHUNK] for i in range(0, len(fw), WRITE_CHUNK)]
    print(f"Writing {len(fw)} bytes in {len(chunks)} chunks...")
    for idx, chunk in enumerate(chunks):
        addr = FLASH_BASE + idx * WRITE_CHUNK
        lines = send_cmd(ser, f'FLASH WRITE 0x{addr:08X} {chunk.hex().upper()}', timeout=5.0)
        if not any('OK FLASH WROTE' in l for l in lines):
            print(f"Write failed at chunk {idx} (0x{addr:08X}):", lines)
            sys.exit(1)

    # 4. Verify Flash
    print("Verifying flash...")
    for idx, chunk in enumerate(chunks):
        addr = FLASH_BASE + idx * WRITE_CHUNK
        lines = send_cmd(ser, f'FLASH VERIFY 0x{addr:08X} {chunk.hex().upper()}', timeout=5.0)
        if not any('OK FLASH VERIFY' in l for l in lines):
            print(f"Verify failed at chunk {idx} (0x{addr:08X}):", lines)
            sys.exit(1)

    print("✓ Flash Programmed & Verified Byte-Perfect!")

    # 5. Reset target to run firmware
    send_cmd(ser, 'DEBUG RUN')
    send_cmd(ser, 'SWD RESET')
    ser.close()

    print("\nTarget reset! Waiting 3 seconds for BluePill USB CDC enumeration...")
    time.sleep(3.0)

if __name__ == '__main__':
    main()
