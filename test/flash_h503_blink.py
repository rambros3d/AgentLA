#!/usr/bin/env python3
"""
Flash and verify blink firmware onto an STM32H503 via AgentLA SWD.
===================================================================
Uses AgentLA's native SWD and STM32H5 flash programming engine:
  - ERASE: 8 KB sector erase or full-chip mass erase
  - WRITE: 128-bit quad-word flash write with 16-byte alignment
  - VERIFY: Readback and byte-level comparison
  - RUN: Target system reset and start execution

Usage:
  python3 test/flash_h503_blink.py
  python3 test/flash_h503_blink.py /dev/ttyACM0
  python3 test/flash_h503_blink.py /dev/ttyACM0 path/to/firmware.bin
"""

import sys
import time
import os
from pathlib import Path
import serial

DEFAULT_BIN = Path("/run/media/sun/halo/bareBoot/test-firmware/blink-h503/blink-h503.bin")
CHUNK_SIZE = 128  # 128 bytes (8 quad-words) per write command


def send_cmd(ser, cmd, timeout=5.0):
    ser.write((cmd.strip() + "\r\n").encode())
    ser.flush()
    start = time.time()
    resp = []
    while time.time() - start < timeout:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        if line:
            resp.append(line)
            if line.startswith("OK") or line.startswith("ERR"):
                break
    return resp


def flash_firmware(port="/dev/ttyACM0", bin_path=DEFAULT_BIN, base_addr=0x08000000):
    bin_path = Path(bin_path)
    if not bin_path.exists():
        print(f"[FAIL] Binary not found at {bin_path}")
        return False

    with open(bin_path, "rb") as f:
        data = f.read()

    size = len(data)
    print(f"[FIRMWARE] Firmware: {bin_path.name} ({size} bytes, ~{(size + 1023) // 1024} KB)")

    print(f"[INFO] Connecting to AgentLA on {port}...")
    try:
        ser = serial.Serial(port, 115200, timeout=1.0)
    except Exception as e:
        print(f"[FAIL] Could not open port {port}: {e}")
        return False

    time.sleep(0.1)
    ser.reset_input_buffer()

    # Step 1: Detect Target
    print("\n[1/5] Detecting target...")
    send_cmd(ser, "SWD RESET")
    resp = send_cmd(ser, "SWD TEST")
    target_found = any("OK SWD TEST PASS" in l for l in resp)
    print(f"  > SWD TEST: {resp}")

    if not target_found:
        print("[FAIL] No target detected. Ensure SWD wiring and external 3.3V power to STM32H503.")
        ser.close()
        return False

    # Step 2: Halt Core
    print("\n[2/5] Halting CPU...")
    resp = send_cmd(ser, "DEBUG HALT")
    print(f"  > DEBUG HALT: {resp}")

    # Step 3: Erase Flash
    # Erase enough 8KB sectors to fit the binary
    erase_size = ((size + 8191) // 8192) * 8192
    print(f"\n[3/5] Erasing {erase_size} bytes ({erase_size // 8192} x 8KB sectors) at 0x{base_addr:08X}...")
    start_t = time.time()
    resp = send_cmd(ser, f"FLASH ERASE {hex(base_addr)} {erase_size}", timeout=10.0)
    print(f"  > FLASH ERASE: {resp}")
    if not any("OK FLASH ERASE" in l for l in resp):
        print("[FAIL] Flash erase failed.")
        ser.close()
        return False
    print(f"  ✓ Erased in {time.time() - start_t:.2f}s")

    # Step 4: Write & Verify
    print(f"\n[4/5] Writing {size} bytes in {CHUNK_SIZE}-byte chunks...")
    start_t = time.time()
    offset = 0
    while offset < size:
        chunk = data[offset : offset + CHUNK_SIZE]
        chunk_addr = base_addr + offset
        hex_data = chunk.hex().upper()

        # Write
        resp = send_cmd(ser, f"FLASH WRITE {hex(chunk_addr)} {hex_data}", timeout=5.0)
        if not any("OK FLASH WROTE" in l for l in resp):
            print(f"\n[FAIL] Write failed at 0x{chunk_addr:08X}: {resp}")
            ser.close()
            return False

        # Verify
        resp = send_cmd(ser, f"FLASH VERIFY {hex(chunk_addr)} {hex_data}", timeout=5.0)
        if not any("OK FLASH VERIFY PASS" in l for l in resp):
            print(f"\n[FAIL] Verify mismatch at 0x{chunk_addr:08X}: {resp}")
            ser.close()
            return False

        offset += len(chunk)
        pct = (offset * 100) // size
        sys.stdout.write(f"\r  Writing & Verifying: [{pct:3d}%] 0x{chunk_addr:08X}")
        sys.stdout.flush()

    duration = time.time() - start_t
    print(f"\n  ✓ Programmed & verified in {duration:.2f}s ({(size / duration) / 1024:.2f} KB/s)")

    # Step 5: Reset & Run
    print("\n[5/5] Resetting and running firmware...")
    send_cmd(ser, "DEBUG RESET")
    send_cmd(ser, "DEBUG RUN")
    print("  ✓ Target rebooted and running!")

    ser.close()
    return True


if __name__ == "__main__":
    p = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
    b = sys.argv[2] if len(sys.argv) > 2 else DEFAULT_BIN
    success = flash_firmware(port=p, bin_path=b)
    sys.exit(0 if success else 1)
