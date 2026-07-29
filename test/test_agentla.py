#!/usr/bin/env python3
"""
AgentLA Host Test Script
Tests the AgentLA logic analyzer by sending commands over serial.

Usage:
  python3 test_agentla.py /dev/ttyACM0

Prerequisites:
  pip install pyserial
"""

import sys
import serial
import time

BAUD = 115200
TIMEOUT = 5

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

def test_identify(ser):
    print("\n=== Test: IDENTIFY ===")
    lines = send_cmd(ser, "IDENTIFY")
    for l in lines:
        print(f"  {l}")
    ok = any("AGENTLA" in l for l in lines)
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_status(ser):
    print("\n=== Test: STATUS ===")
    lines = send_cmd(ser, "STATUS")
    for l in lines:
        print(f"  {l}")
    ok = any("STATUS" in l for l in lines)
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_configure(ser):
    print("\n=== Test: CONFIGURE ===")
    lines = send_cmd(ser, "CONFIGURE channels=6,7,8,9 rate=1000000")
    for l in lines:
        print(f"  {l}")
    ok = any("OK configured" in l for l in lines)
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_raw_capture(ser):
    print("\n=== Test: RAW CAPTURE ===")
    lines = send_cmd(ser, "CAPTURE samples=1024 mode=RAW")
    raw_count = sum(1 for l in lines if l.startswith("RAW"))
    has_done = any(l.startswith("DONE") for l in lines)
    print(f"  Raw samples received: {raw_count}")
    print(f"  DONE received: {has_done}")
    # Print first few raw samples
    for l in lines[:5]:
        if l.startswith("RAW"):
            print(f"  {l}")
    if raw_count > 5:
        print(f"  ... ({raw_count - 5} more)")
    ok = raw_count > 0 and has_done
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_i2c_decode(ser):
    print("\n=== Test: I2C DECODE ===")
    print("  (Ensure ESP32 I2C test is running: send 'i' to ESP32)")
    lines = send_cmd(ser, "CAPTURE samples=4096 mode=DECODED protocol=I2C sda=6 scl=7")
    for l in lines:
        print(f"  {l}")
    has_start = any("START" in l for l in lines)
    has_addr = any("ADDR" in l for l in lines)
    has_data = any("DATA" in l for l in lines)
    has_stop = any("STOP" in l for l in lines)
    ok = has_start and has_addr and has_data and has_stop
    print(f"  START={has_start} ADDR={has_addr} DATA={has_data} STOP={has_stop}")
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_spi_decode(ser):
    print("\n=== Test: SPI DECODE ===")
    print("  (Ensure ESP32 SPI test is running: send 's' to ESP32)")
    lines = send_cmd(ser, "CAPTURE samples=4096 mode=DECODED protocol=SPI cs=9 sclk=7 mosi=6 miso=8")
    for l in lines:
        print(f"  {l}")
    has_cs = any("CS" in l for l in lines)
    has_mosi = any("MOSI" in l for l in lines)
    ok = has_cs and has_mosi
    print(f"  CS={has_cs} MOSI={has_mosi}")
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_uart_decode(ser):
    print("\n=== Test: UART DECODE ===")
    print("  (Ensure ESP32 UART test is running: send 'u' to ESP32)")
    lines = send_cmd(ser, "CAPTURE samples=4096 mode=DECODED protocol=UART rx=6 baud=115200")
    for l in lines:
        print(f"  {l}")
    has_data = any("UART" in l for l in lines)
    print(f"  UART data decoded: {has_data}")
    print(f"  {'PASS' if has_data else 'FAIL'}")
    return has_data

def test_pwm_decode(ser):
    print("\n=== Test: PWM DECODE ===")
    print("  (Ensure ESP32 PWM test is running: send 'p' to ESP32)")
    lines = send_cmd(ser, "CAPTURE samples=4096 mode=DECODED protocol=UART rx=6 baud=115200")
    # Actually test via MEASURE command
    lines = send_cmd(ser, "MEASURE PWM pin=6")
    for l in lines:
        print(f"  {l}")
    ok = any("PWM" in l for l in lines)
    print(f"  {'PASS' if ok else 'FAIL'}")
    return ok

def test_error_handling(ser):
    print("\n=== Test: ERROR HANDLING ===")
    tests = [
        ("FOOBAR", "unknown command"),
        ("CONFIGURE rate=999999999", "exceeds maximum"),
    ]
    all_ok = True
    for cmd, expected in tests:
        lines = send_cmd(ser, cmd)
        ok = any(expected in l.lower() for l in lines)
        print(f"  {cmd} -> {'PASS' if ok else 'FAIL'} ({expected})")
        all_ok = all_ok and ok
    return all_ok

def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <serial_port>")
        print(f"Example: {sys.argv[0]} /dev/ttyACM0")
        sys.exit(1)

    port = sys.argv[1]
    print(f"Connecting to {port} at {BAUD} baud...")

    try:
        ser = serial.Serial(port, BAUD, timeout=TIMEOUT)
    except serial.SerialException as e:
        print(f"Error: {e}")
        sys.exit(1)

    time.sleep(2)  # Wait for device reset
    ser.reset_input_buffer()  # Clear any boot artifacts
    time.sleep(0.5)

    results = {}
    results["identify"] = test_identify(ser)
    results["status"] = test_status(ser)
    results["configure"] = test_configure(ser)
    results["raw_capture"] = test_raw_capture(ser)
    results["error_handling"] = test_error_handling(ser)

    print("\n" + "=" * 50)
    print("RESULTS:")
    for name, passed in results.items():
        print(f"  {name}: {'PASS' if passed else 'FAIL'}")

    total = len(results)
    passed = sum(1 for v in results.values() if v)
    print(f"\n  {passed}/{total} tests passed")

    ser.close()

if __name__ == "__main__":
    main()
