#!/usr/bin/env python3
"""CH32V003 1-Wire SWIO debugging, flash programming, and readback verification."""

import serial
import time
import sys
import struct

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200
PASS_COUNT = 0
FAIL_COUNT = 0


def check(label, condition, detail=""):
    global PASS_COUNT, FAIL_COUNT
    if condition:
        PASS_COUNT += 1
        print(f"  [PASS] {label}")
    else:
        FAIL_COUNT += 1
        print(f"  [FAIL] {label} {detail}")


def send_cmd(ser, cmd, timeout=3.0):
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


def main():
    global PASS_COUNT, FAIL_COUNT

    print("=" * 60)
    print(" CH32V003 1-Wire SWIO + Flash + Readback Verification")
    print("=" * 60)
    print(f" Port: {PORT}")

    try:
        ser = serial.Serial(PORT, BAUD, timeout=2, rtscts=False, dsrdtr=False)
        ser.dtr = True
        ser.rts = True
    except Exception as e:
        print(f"  [FATAL] Cannot open {PORT}: {e}")
        return False

    time.sleep(0.5)

    # Stage 1: IDENTIFY
    print("\nStage 1: Device Handshake")
    lines = send_cmd(ser, "IDENTIFY")
    id_ok = any("AGENTLA" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("AgentLA firmware responds", id_ok)

    # Stage 2: CDC Diagnostics (baseline)
    print("\nStage 2: CDC Diagnostics (baseline)")
    lines = send_cmd(ser, "DIAG CDC")
    for l in lines:
        print(f"  > {l}")
    check("DIAG CDC responds", any("DIAG CDC" in l for l in lines))

    # Stage 3: CH32V003 Detect
    print("\nStage 3: CH32V003 Target Probe (1-Wire SWIO)")
    lines = send_cmd(ser, "CH32 DETECT mcu=ch32v003", timeout=5.0)
    probe_ok = any("OK CH32 DETECT" in l for l in lines)
    flash_size = 0
    page_size = 0
    for l in lines:
        print(f"  > {l}")
        if l.startswith("OK CH32 DETECT"):
            for part in l.split():
                if part.startswith("flash_size="):
                    flash_size = int(part.split("=")[1])
                elif part.startswith("page_size="):
                    page_size = int(part.split("=")[1])

    check("CH32V003 target detected via SWIO", probe_ok)
    check(f"Flash size = {flash_size} bytes", flash_size > 0)
    check(f"Page size = {page_size} bytes", page_size == 64)

    # Stage 4: Erase
    print("\nStage 4: Flash Page Erase (64B @ 0x08000000)")
    lines = send_cmd(ser, "CH32 ERASE 0x08000000 64", timeout=4.0)
    erase_ok = any("OK CH32 ERASED" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("Flash erase successful", erase_ok)

    # Stage 5: Flash Write
    print("\nStage 5: Flash Write (32 Bytes @ 0x08000000)")
    test_payload = bytearray()
    for i in range(8):
        test_payload += struct.pack('<I', 0x12345600 + i)
    hex_payload = test_payload.hex()

    lines = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_payload}", timeout=5.0)
    write_ok = any("OK CH32 FLASH WROTE" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("Flash write successful", write_ok)

    # Stage 6: Readback via CH32 READ
    print("\nStage 6: Flash Readback (CH32 READ)")
    lines = send_cmd(ser, "CH32 READ 0x08000000 32", timeout=3.0)
    readback_ok = False
    for l in lines:
        print(f"  > {l}")
        if l.startswith("FLASH DATA "):
            hex_read = l.replace("FLASH DATA ", "").replace(",", "").strip()
            try:
                read_bytes = bytes.fromhex(hex_read)
                if read_bytes == test_payload:
                    readback_ok = True
                    check(f"Readback matches ({len(read_bytes)} bytes identical)", True)
                else:
                    check("Readback match", False,
                          f"expected {test_payload.hex()}, got {read_bytes.hex()}")
            except Exception as ex:
                check("Readback hex parse", False, str(ex))

    if not readback_ok and not any("FLASH DATA" in l for l in lines):
        check("Readback data received", False, "no FLASH DATA line in response")

    # Stage 7: CDC Diagnostics (after operations)
    print("\nStage 7: CDC Diagnostics (after operations)")
    lines = send_cmd(ser, "DIAG CDC")
    for l in lines:
        print(f"  > {l}")
    check("DIAG CDC responds", any("DIAG CDC" in l for l in lines))

    # Stage 8: Larger write test (if CDC is healthy)
    print("\nStage 8: Larger Write + Readback (64 Bytes)")
    lines = send_cmd(ser, "CH32 ERASE 0x08000000 64", timeout=4.0)
    big_payload = bytearray()
    for i in range(16):
        big_payload += struct.pack('<I', 0xDEADBEE0 + i)
    big_hex = big_payload.hex()

    lines = send_cmd(ser, f"CH32 FLASH 0x08000000 {big_hex}", timeout=5.0)
    big_write_ok = any("OK CH32 FLASH WROTE" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("64-byte flash write successful", big_write_ok)

    lines = send_cmd(ser, "CH32 READ 0x08000000 64", timeout=3.0)
    big_read_ok = False
    for l in lines:
        if l.startswith("FLASH DATA "):
            hex_read = l.replace("FLASH DATA ", "").replace(",", "").strip()
            try:
                read_bytes = bytes.fromhex(hex_read)
                if read_bytes == big_payload:
                    big_read_ok = True
                    check(f"64-byte readback matches", True)
                else:
                    check("64-byte readback match", False,
                          f"mismatch at byte {next((i for i in range(min(len(read_bytes), len(big_payload))) if read_bytes[i] != big_payload[i]), -1)}")
            except Exception as ex:
                check("64-byte readback parse", False, str(ex))

    ser.close()

    print()
    print("=" * 60)
    total = PASS_COUNT + FAIL_COUNT
    print(f" RESULTS: {PASS_COUNT}/{total} passed, {FAIL_COUNT}/{total} failed")
    if FAIL_COUNT == 0:
        print(" ALL TESTS PASSED")
    else:
        print(f" {FAIL_COUNT} TEST(S) FAILED")
    print("=" * 60)
    return FAIL_COUNT == 0


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
