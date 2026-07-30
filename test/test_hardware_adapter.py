#!/usr/bin/env python3
"""
AgentLA Adapter Mode Hardware Test

Tests I2C and UART adapter functionality with real hardware.

Usage:
  python3 test/test_hardware_adapter.py /dev/ttyACM0

Prerequisites:
  pip install pyserial
  RP2350 AgentLA device connected with I2C slave (ESP32) on GPIO0/GP1
"""

import sys
import time
import serial

BAUD = 115200
TIMEOUT = 5

def send_cmd(ser, cmd, wait_for_echo=True):
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

def wait_for_adapter_lines(ser, max_lines=10):
    """Wait for IDENTIFY response from adapter port."""
    lines = []
    deadline = time.time() + TIMEOUT
    while time.time() < deadline and len(lines) < max_lines:
        line = ser.readline().decode().strip()
        if line:
            lines.append(line)
            if line.startswith("DONE") or line.startswith("ERR"):
                break
    return lines

def test_identify(ser):
    print("\n=== Test: IDENTIFY ===")
    lines = send_cmd(ser, "IDENTIFY")
    for line in lines:
        print(f"  {line}")
    role_line = next((l for l in lines if "ADAPTER" in l.upper() or "ANALYZER" in l), None)
    if role_line:
        print(f"  Role detected: {'ADAPTER' if 'ADAPTER' in role_line else 'ANALYZER'}")
    return True

def test_i2c_write(ser):
    print("\n=== Test: I2C WRITE ===")
    lines = send_cmd(ser, "I2C_WRITE addr=0x48 reg=0x02 data=01")
    for line in lines:
        print(f"  {line}")
    return any("OK I2C WRITE" in l for l in lines)

def test_i2c_read(ser):
    print("\n=== Test: I2C READ ===")
    # First write something to read back
    send_cmd(ser, "I2C_WRITE addr=0x48 reg=0x02 data=01")
    time.sleep(0.1)

    lines = send_cmd(ser, "I2C_READ addr=0x48 reg=0x02 len=1")
    for line in lines:
        print(f"  {line}")
    return any("I2C DATA" in l for l in lines)

def test_i2c_nack(ser):
    print("\n=== Test: I2C NACK handling ===")
    lines = send_cmd(ser, "I2C_READ addr=0x7F reg=0x00 len=1")
    for line in lines:
        print(f"  {line}")
    return any("ERR" in l and "NACK" in l for l in lines)

def test_i2c_clear(ser):
    print("\n=== Test: I2C CLEAR ===")
    lines = send_cmd(ser, "I2C_CLEAR")
    for line in lines:
        print(f"  {line}")
    return any("OK I2C BUS CLEARED" in l for l in lines)

def test_uart_write(ser):
    print("\n=== Test: UART WRITE ===")
    lines = send_cmd(ser, "UART_WRITE baud=115200 data=48,65,6C,6C,6F")
    for line in lines:
        print(f"  {line}")
    return any("OK UART WROTE" in l for l in lines)

def test_uart_read(ser):
    print("\n=== Test: UART READ ===")
    # Send data first (loopback or echo from ESP32)
    send_cmd(ser, "UART_WRITE baud=115200 data=41,42,43")
    time.sleep(0.2)

    lines = send_cmd(ser, "UART_READ baud=115200 len=10 timeout=500")
    for line in lines:
        print(f"  {line}")
    return any("UART DATA" in l for l in lines)

def test_uart_baud_validation(ser):
    print("\n=== Test: UART baud validation ===")
    lines = send_cmd(ser, "UART_WRITE baud=100 data=41")
    for line in lines:
        print(f"  {line}")
    return any("ERR invalid baud rate" in l for l in lines)

def run_adapter_tests(port):
    print(f"AgentLA Adapter Hardware Test on {port}")
    print("=" * 50)

    try:
        ser = serial.Serial(port, BAUD, timeout=TIMEOUT)
        time.sleep(0.5)
        ser.reset_input_buffer()
        ser.reset_output_buffer()
    except serial.SerialException as e:
        print(f"ERROR: Cannot open {port}: {e}")
        return False

    # First identify the device
    if not test_identify(ser):
        print("IDENTIFY failed, aborting")
        ser.close()
        return False

    results = {}

    # I2C tests (require ESP32 I2C slave on GPIO0/GP1)
    print("\n--- I2C Tests (requires ESP32 I2C slave on GPIO0/GP1) ---")
    results["I2C WRITE"] = test_i2c_write(ser)
    results["I2C READ"] = test_i2c_read(ser)
    results["I2C NACK"] = test_i2c_nack(ser)
    results["I2C CLEAR"] = test_i2c_clear(ser)

    # UART tests
    print("\n--- UART Tests ---")
    results["UART WRITE"] = test_uart_write(ser)
    results["UART READ"] = test_uart_read(ser)
    results["UART baud validation"] = test_uart_baud_validation(ser)

    ser.close()

    print("\n" + "=" * 50)
    print("RESULTS SUMMARY")
    print("=" * 50)
    for test_name, passed in results.items():
        status = "PASS" if passed else "FAIL"
        print(f"  [{status}] {test_name}")

    all_passed = all(results.values())
    print(f"\nOverall: {'ALL TESTS PASSED' if all_passed else 'SOME TESTS FAILED'}")
    return all_passed


if __name__ == "__main__":
    port = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
    run_adapter_tests(port)