#!/usr/bin/env python3
"""
Hardware Verification Suite for CH32V203 2-Wire RV-SWD Debugging & Flash Programming
Target: AgentLA RP2350 Programmer + CH32V203 Target Board
"""

import os
import sys
import time
import subprocess
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

def cleanup_port_locks(port):
    """Clean up any zombie process locks on serial port."""
    try:
        subprocess.run(["fuser", "-k", port], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.3)
    except Exception:
        pass

def send_cmd(ser, cmd, timeout=3.0):
    """Send serial command to AgentLA and return response lines."""
    ser.reset_input_buffer()
    ser.write((cmd.strip() + '\r\n').encode())
    ser.flush()
    time.sleep(0.2)
    
    deadline = time.time() + timeout
    lines = []
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors='replace').strip()
            if line:
                lines.append(line)
                if line.startswith("OK") or line.startswith("ERR"):
                    break
        else:
            time.sleep(0.05)
    return lines

def run_hardware_verification():
    print("============================================================")
    print(" AgentLA CH32V203 2-Wire RV-SWD Hardware Verification Suite")
    print("============================================================")
    print(f"Target Serial Port: {PORT}")

    cleanup_port_locks(PORT)

    try:
        ser = serial.Serial(PORT, BAUD, timeout=2, rtscts=False, dsrdtr=False)
        ser.dtr = True
        ser.rts = True
    except Exception as e:
        print(f"❌ Failed to open serial port {PORT}: {e}")
        return False

    time.sleep(0.5)

    # ------------------------------------------------------------------------
    # Stage 1: Device Handshake (IDENTIFY)
    # ------------------------------------------------------------------------
    print("\n--- Stage 1: AgentLA Firmware Handshake ---")
    lines = send_cmd(ser, "IDENTIFY")
    id_ok = any("AGENTLA" in l for l in lines)
    for l in lines: print(f"  {l}")
    if id_ok:
        print("  ✓ Stage 1 Passed: AgentLA Firmware Handshake Successful")
    else:
        print("  ❌ Stage 1 Failed: Device did not return AGENTLA banner")

    # ------------------------------------------------------------------------
    # Stage 2: Target Probe (CH32 DETECT mode=2wire)
    # ------------------------------------------------------------------------
    print("\n--- Stage 2: Target Probe & ESIG Match (2-Wire RV-SWD) ---")
    lines = send_cmd(ser, "CH32 DETECT mode=2wire", timeout=3.0)
    probe_ok = any("OK CH32 DETECT" in l for l in lines)
    for l in lines: print(f"  {l}")
    if probe_ok:
        print("  ✓ Stage 2 Passed: CH32V203 ESIG Probe Successful")
    else:
        print("  ❌ Stage 2 Failed: CH32V203 Target Probe Failed")

    # ------------------------------------------------------------------------
    # Stage 3: Sector Erase (CH32 ERASE 0x08000000 128)
    # ------------------------------------------------------------------------
    print("\n--- Stage 3: Flash Sector Erase (128 Bytes at 0x08000000) ---")
    lines = send_cmd(ser, "CH32 ERASE 0x08000000 128", timeout=4.0)
    erase_ok = any("OK CH32 ERASED" in l for l in lines)
    for l in lines: print(f"  {l}")
    if erase_ok:
        print("  ✓ Stage 3 Passed: Flash Page Erase Successful")
    else:
        print("  ❌ Stage 3 Failed: Flash Erase Command Failed")

    # ------------------------------------------------------------------------
    # Stage 4: Fast Mode Flash Write (CH32 FLASH 0x08000000 <payload>)
    # ------------------------------------------------------------------------
    print("\n--- Stage 4: Fast Mode Flash Write (128 Bytes Payload) ---")
    payload_words = [0x12345678, 0x9ABCDEF0, 0x00112233, 0x44556677] * 8  # 32 words = 128 bytes
    test_payload = bytearray()
    for w in payload_words:
        test_payload += w.to_bytes(4, byteorder='little')
    hex_payload = test_payload.hex()

    lines = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_payload}", timeout=5.0)
    write_ok = any("OK CH32 FLASH WROTE" in l for l in lines)
    for l in lines: print(f"  {l}")
    if write_ok:
        print("  ✓ Stage 4 Passed: Fast Page Write Successful")
    else:
        print("  ❌ Stage 4 Failed: Flash Write Command Failed")

    # ------------------------------------------------------------------------
    # Stage 5: Readback & Payload Parity Verification
    # ------------------------------------------------------------------------
    print("\n--- Stage 5: Flash Readback & Byte Parity Verification ---")
    lines = send_cmd(ser, "CH32 READ 0x08000000 128", timeout=3.0)
    read_ok = False
    for l in lines:
        print(f"  {l}")
        if l.startswith("FLASH DATA "):
            hex_read = l.replace("FLASH DATA ", "").replace(",", "").strip()
            try:
                read_bytes = bytes.fromhex(hex_read)
                if read_bytes == test_payload:
                    read_ok = True
                    print(f"  ✓ Byte Parity Verification Match: {len(read_bytes)} bytes match payload exactly!")
                else:
                    print(f"  ❌ Mismatch: Read {len(read_bytes)} bytes, expected {len(test_payload)} bytes")
            except Exception as ex:
                print(f"  ❌ Hex parsing error: {ex}")

    ser.close()

    print("\n============================================================")
    all_passed = id_ok and probe_ok and erase_ok and write_ok and read_ok
    if all_passed:
        print(" SUCCESS: ALL CH32V203 HARDWARE VERIFICATION STAGES PASSED! ✓")
    else:
        print(" FAILURE: ONE OR MORE VERIFICATION STAGES FAILED")
    print("============================================================")
    return all_passed

if __name__ == "__main__":
    success = run_hardware_verification()
    sys.exit(0 if success else 1)
