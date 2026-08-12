#!/usr/bin/env python3
"""
Raw passthrough test for AN3155 STM32 UART bootloader debugging.

This script tests the raw passthrough mode to verify communication
with Blue Pill STM32F103 UART bootloader directly.

Escape sequence: +++\\r\\n exits raw passthrough mode.

Hardware setup:
- RP2350 GPIO8 (TX) → Blue Pill PA10 (UART1 RX)
- RP2350 GPIO9 (RX) → Blue Pill PA9 (UART1 TX)
- Blue Pill BOOT0 = HIGH (3.3V) for UART bootloader mode
"""
import serial
import time
import sys

RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200
TIMEOUT = 5


def send_cmd(ser, cmd, wait=0.5):
    """Send a command to AgentLA and return response lines (skipping echo)."""
    ser.reset_input_buffer()
    ser.write((cmd + "\r\n").encode())
    time.sleep(wait)
    lines = []
    while ser.in_waiting:
        line = ser.readline().decode(errors="replace").strip()
        if line:
            # Skip the echo line (starts with "> ")
            if not line.startswith("> "):
                lines.append(line)
    return lines


def drain(ser, timeout=0.5):
    """Drain any pending data from serial buffer."""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            ser.read(ser.in_waiting)
        time.sleep(0.01)


def exit_raw_passthrough(ser, timeout=2.0):
    """Exit raw passthrough using escape sequence +++\\r\\n."""
    print("Sending escape sequence +++\\r\\n...")
    ser.write(b'+++\r\n')
    time.sleep(0.5)
    
    # Read response looking for "OK RAW OFF"
    resp = b''
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            data = ser.read(ser.in_waiting)
            resp += data
            if b'OK RAW OFF' in resp:
                print("  Exited raw passthrough successfully!")
                return True
        time.sleep(0.01)
    
    text = resp.decode(errors="replace").strip()
    if text:
        print(f"  Response: {text}")
    return b'OK RAW OFF' in resp


def test_sync_raw(ser):
    """Send raw 0x7F SYNC and look for 0x79 ACK from STM32 bootloader."""
    print("\n--- Raw 0x7F SYNC Test ---")
    
    # Enable raw passthrough mode
    print("Enabling raw passthrough...")
    lines = send_cmd(ser, "PROG RAW ON", wait=0.3)
    for l in lines:
        print(f"  {l}")
    
    if not any("OK RAW ON" in l for l in lines):
        print("  Failed to enable raw passthrough!")
        return False
    
    # Drain any pending data
    drain(ser, 0.2)
    ser.reset_input_buffer()
    
    # Send raw 0x7F SYNC byte
    print("Sending raw 0x7F SYNC byte...")
    ser.write(b'\x7f')
    
    # Wait for response
    time.sleep(0.5)
    
    # Read response
    response = b''
    deadline = time.time() + 2.0
    while time.time() < deadline:
        if ser.in_waiting:
            data = ser.read(ser.in_waiting)
            response += data
        time.sleep(0.01)
    
    print(f"Response bytes: {response.hex() if response else '(empty)'}")
    print(f"Response length: {len(response)} bytes")
    
    # Exit raw passthrough using escape sequence
    exit_raw_passthrough(ser)
    
    # Check if we got ACK (0x79)
    if b'\x79' in response:
        print("\n✓ SUCCESS: Got 0x79 ACK from STM32 bootloader!")
        print("  Blue Pill UART bootloader is responding correctly.")
        return True
    elif len(response) == 0:
        print("\n✗ FAILED: No response received.")
        print("  Check:")
        print("  - Blue Pill BOOT0 is HIGH (3.3V)")
        print("  - Wiring: RP2350 GPIO8→Blue Pill PA10, GPIO9→PA9")
        print("  - Blue Pill is powered")
        return False
    else:
        print(f"\n✗ FAILED: Unexpected response: {response.hex()}")
        return False


def test_an3155_commands(ser):
    """Test basic AN3155 commands via raw passthrough."""
    print("\n--- AN3155 Command Tests ---")
    
    # Enable raw passthrough
    lines = send_cmd(ser, "PROG RAW ON", wait=0.3)
    for l in lines:
        print(f"  {l}")
    
    drain(ser, 0.2)
    ser.reset_input_buffer()
    
    results = {}
    
    # Test 1: GetVersion (0x01)
    print("\n1. GetVersion (0x01)...")
    ser.write(b'\x01\xfe')  # Command + complement
    time.sleep(0.3)
    resp = b''
    deadline = time.time() + 1.0
    while time.time() < deadline:
        if ser.in_waiting:
            resp += ser.read(ser.in_waiting)
        time.sleep(0.01)
    print(f"   Response: {resp.hex() if resp else '(empty)'}")
    results['GetVersion'] = resp
    
    drain(ser, 0.1)
    
    # Test 2: GetID (0x02)
    print("\n2. GetID (0x02)...")
    ser.write(b'\x02\xfd')  # Command + complement
    time.sleep(0.3)
    resp = b''
    deadline = time.time() + 1.0
    while time.time() < deadline:
        if ser.in_waiting:
            resp += ser.read(ser.in_waiting)
        time.sleep(0.01)
    print(f"   Response: {resp.hex() if resp else '(empty)'}")
    results['GetID'] = resp
    
    # Exit raw passthrough using escape sequence
    exit_raw_passthrough(ser)
    
    # Analyze results
    print("\n--- Analysis ---")
    for cmd_name, resp in results.items():
        if resp and b'\x79' in resp:
            print(f"✓ {cmd_name}: Got ACK (0x79)")
        elif resp and b'\x1f' in resp:
            print(f"✗ {cmd_name}: Got NACK (0x1F)")
        elif len(resp) > 0:
            print(f"? {cmd_name}: Got {len(resp)} bytes: {resp.hex()}")
        else:
            print(f"✗ {cmd_name}: No response")
    
    return any(b'\x79' in r for r in results.values())


def main():
    print("=== Raw Passthrough Test for AN3155 Debugging ===")
    print(f"Port: {RP_PORT}")
    print(f"Baud: {BAUD}")
    
    print("\nHardware prerequisites:")
    print("  - Blue Pill BOOT0 = HIGH (3.3V)")
    print("  - Blue Pill nRESET pulsed after BOOT0 set")
    print("  - Wiring: RP2350 GPIO8→PA10, GPIO9→PA9")
    print()
    
    try:
        ser = serial.Serial(RP_PORT, BAUD, timeout=TIMEOUT)
    except serial.SerialException as e:
        print(f"Error opening {RP_PORT}: {e}")
        return 1
    
    time.sleep(1)
    drain(ser, 2.0)
    
    # Enter programmer mode
    print("\nEntering programmer mode...")
    lines = send_cmd(ser, "MODE PROG ON", wait=0.5)
    for l in lines:
        print(f"  {l}")
    
    if not any("OK PROG MODE ON" in l for l in lines):
        print("Failed to enter programmer mode!")
        ser.close()
        return 1
    
    # Run tests
    sync_ok = test_sync_raw(ser)
    
    if sync_ok:
        # If SYNC worked, try the other commands
        commands_ok = test_an3155_commands(ser)
    
    # Exit programmer mode
    print("\nExiting programmer mode...")
    lines = send_cmd(ser, "MODE PROG OFF", wait=0.3)
    for l in lines:
        print(f"  {l}")
    
    ser.close()
    
    print("\n=== Test Complete ===")
    if sync_ok:
        print("Raw passthrough is working correctly!")
        print("You can now implement full AN3155 protocol using raw bytes.")
    else:
        print("Raw passthrough test failed. Check hardware connections.")
    
    return 0 if sync_ok else 1


if __name__ == "__main__":
    sys.exit(main())
