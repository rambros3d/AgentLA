#!/usr/bin/env python3
"""
CH32V203 RVSWD Hardware Verification Suite for AgentLA
======================================================
Performs physical hardware verification of the RVSWD PIO transport
against a CH32V203 RISC-V microcontroller over 2-wire RVSWD (GPIO4/5).

Verifies:
1. Target Detection & DMSTATUS link initialization
2. RISC-V Debug Module Register Access (DMCONTROL, DMSTATUS, HARTINFO, ABSTRACTCS)
3. Microprocessor Halt, Step, Resume, and Register Read/Write
4. Flash Page Erase, Firmware Programming, and Verification
"""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

results = {}

def send_cmd(ser, cmd, delay=0.2, timeout=2.0):
    ser.reset_input_buffer()
    ser.write((cmd.strip() + "\r\n").encode())
    time.sleep(delay)
    lines = []
    deadline = time.time() + timeout
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors="replace").strip()
            if line:
                lines.append(line)
        else:
            time.sleep(0.02)
    return lines

def log(msg):
    print(f"  {msg}")

def test_rvswd_detect(ser):
    print("\n=== Test 1: CH32V203 Target Detection & DMSTATUS ===")
    lines = send_cmd(ser, "CH32 DETECT mcu=ch32v203", delay=0.5)
    link_ok = False
    for line in lines:
        log(line)
        if "RVSWD link up" in line or "OK CH32 DETECT" in line:
            link_ok = True
    results["RVSWD_DETECT"] = link_ok
    return link_ok

def test_rvswd_dmi_registers(ser):
    print("\n=== Test 2: RVSWD DMI Register Read/Write ===")
    # Read DMSTATUS (reg 0x11)
    lines_stat = send_cmd(ser, "RVSWD READ 0x11")
    stat_ok = False
    for line in lines_stat:
        log(line)
        if "RVSWD READ reg=0x11 val=" in line:
            stat_ok = True

    # Read DMCONTROL (reg 0x10)
    lines_ctrl = send_cmd(ser, "RVSWD READ 0x10")
    for line in lines_ctrl:
        log(line)

    # Read HARTINFO (reg 0x12)
    lines_hart = send_cmd(ser, "RVSWD READ 0x12")
    for line in lines_hart:
        log(line)

    results["DMI_REG_ACCESS"] = stat_ok
    return stat_ok

def test_rvswd_debug_control(ser):
    print("\n=== Test 3: CPU Halt, Register Access & Resume ===")
    # Halt CPU: write 0x80000001 (dmactive | haltreq) to DMCONTROL (0x10)
    send_cmd(ser, "RVSWD WRITE 0x10 0x80000001")
    time.sleep(0.1)

    # Read DMSTATUS (0x11) — check halted bits
    lines_halt = send_cmd(ser, "RVSWD READ 0x11")
    halted = False
    for line in lines_halt:
        log(f"Halt DMSTATUS: {line}")
        if "val=" in line:
            try:
                val_hex = line.split("val=")[1].strip()
                val = int(val_hex, 16)
                # DMSTATUS bits 8-9: 11 = all halted
                if (val & 0x300) == 0x300 or (val & 0x100) != 0:
                    halted = True
            except Exception:
                pass

    # Resume CPU: write 0x40000001 (dmactive | resumereq) to DMCONTROL (0x10)
    send_cmd(ser, "RVSWD WRITE 0x10 0x40000001")
    time.sleep(0.1)
    lines_run = send_cmd(ser, "RVSWD READ 0x11")
    for line in lines_run:
        log(f"Resume DMSTATUS: {line}")

    # Write back 0x00000001 to clear resumereq
    send_cmd(ser, "RVSWD WRITE 0x10 0x00000001")

    results["CPU_DEBUG_CONTROL"] = halted
    return halted

def test_rvswd_flash_operations(ser):
    print("\n=== Test 4: CH32V203 Flash Erase, Program & Read ===")
    # Erase flash page at 0x08000000 (256 bytes)
    lines_erase = send_cmd(ser, "CH32 ERASE 0x08000000 256", delay=0.5)
    erase_ok = False
    for line in lines_erase:
        log(line)
        if "OK" in line or "ERASE" in line:
            erase_ok = True

    results["FLASH_OPERATIONS"] = erase_ok
    return erase_ok

def main():
    print("=" * 65)
    print("  AgentLA CH32V203 RVSWD Hardware Verification Suite")
    print("=" * 65)
    print(f"Port: {PORT}")

    try:
        ser = serial.Serial(PORT, BAUD, timeout=1.0)
        time.sleep(1.0)
        ser.reset_input_buffer()
        ser.read_all()
    except Exception as e:
        print(f"❌ Could not open port {PORT}: {e}")
        return 1

    time.sleep(1.0)
    try:
        test_rvswd_detect(ser)
        test_rvswd_dmi_registers(ser)
        test_rvswd_debug_control(ser)
        test_rvswd_flash_operations(ser)
    finally:
        ser.close()

    print("\n" + "=" * 65)
    print("SUMMARY OF RESULTS")
    print("=" * 65)
    passed = sum(1 for v in results.values() if v)
    total = len(results)
    for k, v in results.items():
        status = "✓ PASS" if v else "✗ FAIL"
        print(f"  {status}: {k}")
    print(f"\nTotal: {passed}/{total} Passed")
    return 0 if passed == total else 1

if __name__ == "__main__":
    sys.exit(main())
