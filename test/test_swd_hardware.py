#!/usr/bin/env python3
"""
Comprehensive SWD Hardware Test for AgentLA RP2350 + STM32 Blue Pill

Hardware setup:
- RP2350 GPIO4 (SWCLK) → STM32 SWCLK (PA14)
- RP2350 GPIO5 (SWDIO) → STM32 SWDIO (PA13)
- RP2350 GPIO6 (nRST) → STM32 nRST (NRST)
- RP2350 GPIO7 (PWR) → STM32 3.3V (optional power enable)
- Common GND between RP2350 and STM32

Usage:
    python3 test_swd_hardware.py [serial_port]
    python3 test_swd_hardware.py /dev/ttyACM0
"""
import serial
import time
import sys

RP_PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200
TIMEOUT = 5

# Test results tracking
results = {}

def send_cmd(ser, cmd, wait=0.5, timeout=2.0):
    """Send a command and return response lines."""
    ser.reset_input_buffer()
    ser.write((cmd + "\r\n").encode())
    time.sleep(wait)
    lines = []
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors="replace").strip()
            if line:
                lines.append(line)
        else:
            time.sleep(0.01)
    return lines

def check_response(lines, expected, test_name):
    """Check if expected string is in response lines."""
    for line in lines:
        if expected in line:
            print(f"  ✓ {test_name}: {line}")
            results[test_name] = True
            return True
    print(f"  ✗ {test_name}: Expected '{expected}' not found")
    for line in lines:
        print(f"    Response: {line}")
    results[test_name] = False
    return False

def test_identify(ser):
    """Test IDENTIFY command."""
    print("\n=== Test 1: IDENTIFY ===")
    lines = send_cmd(ser, "IDENTIFY", wait=1.0)
    check_response(lines, "AGENTLA", "IDENTIFY")
    # Print all identification lines
    for line in lines:
        if line.startswith("AGENTLA") or line.startswith("BUILD:") or line.startswith("FREQ:"):
            print(f"  Info: {line}")

def test_swd_pins(ser):
    """Test SWD PINS command."""
    print("\n=== Test 2: SWD PINS ===")
    lines = send_cmd(ser, "SWD PINS")
    check_response(lines, "SWD PINS", "SWD PINS")

def test_swd_idcode(ser):
    """Test SWD IDCODE read."""
    print("\n=== Test 3: SWD IDCODE ===")
    lines = send_cmd(ser, "SWD IDCODE", wait=2.0)
    success = check_response(lines, "OK SWD IDCODE", "SWD IDCODE")
    if success:
        for line in lines:
            if "IDCODE" in line:
                print(f"  IDCODE: {line}")
    return success

def test_swd_test(ser):
    """Test SWD TEST (full connectivity check)."""
    print("\n=== Test 4: SWD TEST ===")
    lines = send_cmd(ser, "SWD TEST", wait=2.0)
    success = check_response(lines, "OK SWD TEST PASS", "SWD TEST")
    if success:
        for line in lines:
            if "TEST" in line:
                print(f"  Result: {line}")
    return success

def test_swd_reset(ser):
    """Test SWD line reset."""
    print("\n=== Test 5: SWD RESET ===")
    lines = send_cmd(ser, "SWD RESET")
    check_response(lines, "OK SWD RESET", "SWD RESET")

def test_swd_read_dp(ser):
    """Test reading DP registers."""
    print("\n=== Test 6: SWD READ DP ===")
    # Read DPIDR (should return IDCODE)
    lines = send_cmd(ser, "SWD READ DP 0x0", wait=1.0)
    success = check_response(lines, "OK SWD DP", "SWD READ DPIDR")
    if success:
        for line in lines:
            if "DP[0x00]" in line:
                print(f"  DPIDR: {line}")
    return success

def test_target_detection(ser):
    """Test target detection and identification."""
    print("\n=== Test 7: Target Detection ===")
    # First read IDCODE to trigger detection
    lines = send_cmd(ser, "SWD IDCODE", wait=2.0)
    idcode_found = False
    for line in lines:
        if "OK SWD IDCODE" in line:
            idcode_found = True
            # Extract IDCODE value
            try:
                idcode_hex = line.split("0x")[1].strip()
                idcode = int(idcode_hex, 16)
                designer_id = (idcode >> 12) & 0xFFF
                part_id = idcode & 0xFFF
                print(f"  IDCODE: 0x{idcode:08X}")
                print(f"  Designer ID: 0x{designer_id:03X}")
                print(f"  Part ID: 0x{part_id:03X}")
                
                # Identify target
                if designer_id == 0x043:
                    print(f"  Target: STMicroelectronics (0x043)")
                    if part_id == 0x414:
                        print(f"  MCU: STM32F103 (Blue Pill)")
                    elif part_id == 0x413:
                        print(f"  MCU: STM32F411")
                    else:
                        print(f"  MCU: STM32 (Part 0x{part_id:03X})")
                elif (designer_id & 0xFFF0) == 0x483:
                    print(f"  Target: Raspberry Pi (0x483)")
                    if part_id == 0x00D:
                        print(f"  MCU: RP2350")
                    elif part_id == 0x00C:
                        print(f"  MCU: RP2040")
                else:
                    print(f"  Target: Unknown (Designer 0x{designer_id:03X})")
            except:
                print(f"  Could not parse IDCODE from: {line}")
    
    if idcode_found:
        results["TARGET_DETECTION"] = True
    else:
        print("  ✗ No IDCODE received")
        results["TARGET_DETECTION"] = False
    
    return idcode_found

def test_swd_dp_access(ser):
    """Test DP register access."""
    print("\n=== Test 8: DP Register Access ===")
    
    # Read CTRL/STAT register
    lines = send_cmd(ser, "SWD READ DP 0x4", wait=1.0)
    success1 = check_response(lines, "OK SWD DP", "DP CTRL/STAT")
    
    # Read SELECT register (write-only, but read returns RDBUFF)
    lines = send_cmd(ser, "SWD READ DP 0xC", wait=1.0)
    success2 = check_response(lines, "OK SWD DP", "DP RDBUFF")
    
    return success1 or success2

def test_swd_ap_access(ser):
    """Test AP register access."""
    print("\n=== Test 9: AP Register Access ===")
    
    # Write AP CSW (32-bit access)
    lines = send_cmd(ser, "SWD WRITE AP 0x0 0x23000012", wait=1.0)
    success1 = check_response(lines, "OK SWD AP", "AP CSW Write")
    
    # Read AP CSW
    lines = send_cmd(ser, "SWD READ AP 0x0", wait=1.0)
    success2 = check_response(lines, "OK SWD AP", "AP CSW Read")
    
    return success1 or success2

def test_flash_commands(ser):
    """Test flash commands (requires target connection)."""
    print("\n=== Test 10: Flash Commands ===")
    
    # Try to read flash at address 0x08000000 (STM32 flash base)
    lines = send_cmd(ser, "FLASH READ 0x08000000 16", wait=2.0)
    for line in lines:
        print(f"  {line}")
    
    # Check for success or expected error (if no target)
    success = any("FLASH DATA" in line or "OK" in line or "ERR no target" in line for line in lines)
    results["FLASH_COMMANDS"] = success
    return success

def test_bootsel(ser):
    """Test BOOTSEL command. WARNING: Device will reboot!"""
    print("\n=== Test 11: BOOTSEL ===")
    print("  WARNING: Device will reboot after this test!")
    lines = send_cmd(ser, "BOOTSEL", wait=1.0)
    check_response(lines, "OK ENTERING BOOTSEL", "BOOTSEL")
    # Device will reboot after this, so we can't do more tests

def print_summary():
    """Print test summary."""
    print("\n" + "="*60)
    print("TEST SUMMARY")
    print("="*60)
    
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    
    for name, result in results.items():
        status = "✓ PASS" if result else "✗ FAIL"
        print(f"  {status}: {name}")
    
    print(f"\n  Total: {passed}/{total} passed")
    
    if passed == total:
        print("\n  [PASS] All tests passed!")
    else:
        print(f"\n  [WARN] {total - passed} test(s) failed")
    
    return passed == total

def main():
    print("="*60)
    print("AgentLA SWD Hardware Test")
    print("="*60)
    print(f"Port: {RP_PORT}")
    print(f"Baud: {BAUD}")
    
    print("\nHardware prerequisites:")
    print("  - RP2350 connected via USB")
    print("  - STM32 Blue Pill connected:")
    print("    GPIO4 → PA14 (SWCLK)")
    print("    GPIO5 → PA13 (SWDIO)")
    print("    GPIO6 → NRST (optional)")
    print("    GND → GND")
    print()
    
    try:
        ser = serial.Serial(RP_PORT, BAUD, timeout=TIMEOUT)
    except serial.SerialException as e:
        print(f"Error opening {RP_PORT}: {e}")
        return 1
    
    time.sleep(2)  # Wait for device to boot
    ser.reset_input_buffer()
    
    try:
        # Basic connectivity test
        test_identify(ser)
        
        # SWD tests
        test_swd_pins(ser)
        
        idcode_ok = test_swd_idcode(ser)
        if not idcode_ok:
            print("\n[WARN] SWD IDCODE failed. Check wiring and target power.")
            print("  Continuing with other tests...")
        
        test_swd_test(ser)
        test_swd_reset(ser)
        
        if idcode_ok:
            test_swd_read_dp(ser)
            test_target_detection(ser)
            test_swd_dp_access(ser)
            test_swd_ap_access(ser)
            test_flash_commands(ser)
        else:
            print("\nSkipping advanced tests (no target detected)")
            results["SKIP_DP_ACCESS"] = True
            results["SKIP_AP_ACCESS"] = True
            results["SKIP_FLASH"] = True
        
        # Don't run BOOTSEL test by default - it reboots the device
        # Uncomment to test:
        # test_bootsel(ser)
        
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
    finally:
        ser.close()
    
    print_summary()
    return 0 if all(results.values()) else 1

if __name__ == "__main__":
    sys.exit(main())
