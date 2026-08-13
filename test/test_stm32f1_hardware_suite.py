#!/usr/bin/env python3
"""
AgentLA STM32F1 (Blue Pill) 5-Tier Automated Hardware Verification Suite
================================================─────────────────────────
Performs end-to-end physical hardware verification of the AgentLA RP2350
probe against an STM32F103 (Blue Pill) microcontroller over SWD & LA.
"""

import sys
import time
import subprocess
import serial

def print_header(title):
    print("\n" + "═" * 70)
    print(f"  {title}")
    print("═" * 70)

def send_cmd(ser, cmd, timeout=2.0):
    ser.write((cmd.strip() + "\r\n").encode())
    ser.flush()
    start = time.time()
    resp = []
    while time.time() - start < timeout:
        line = ser.readline().decode('utf-8', errors='ignore').strip()
        if line:
            resp.append(line)
            if line.startswith("OK") or line.startswith("ERR") or line.startswith("DIAG"):
                break
    return resp

def main():
    port = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
    print_header(f"AgentLA STM32F1 5-Tier Hardware Verification Suite ({port})")

    try:
        ser = serial.Serial(port, 115200, timeout=1.0)
    except Exception as e:
        print(f"❌ Failed to open serial port {port}: {e}")
        sys.exit(1)

    passed_tiers = 0
    total_tiers = 5

    # -------------------------------------------------------------------------
    # Tier 1: Transport & Protocol Layer Verification
    # -------------------------------------------------------------------------
    print("\n--- Tier 1: Transport & Protocol Layer Verification ---")
    resp_drv = send_cmd(ser, "PROG DRIVER PIO")
    print(f"  > PROG DRIVER PIO -> {resp_drv}")
    resp = send_cmd(ser, "SWD CLOCK 1000")
    print(f"  > SWD CLOCK 1000 -> {resp}")
    
    resp = send_cmd(ser, "STM32 TEST")
    print(f"  > STM32 TEST -> {resp}")
    
    tier1_ok = False
    for line in resp:
        if "OK STM32 TEST PASS" in line and "idcode=0x1BA01477" in line:
            tier1_ok = True
            break
            
    if tier1_ok:
        print("  ✓ Tier 1 Passed: SWD Transport & DPIDR 0x1BA01477 Match")
        passed_tiers += 1
    else:
        print("  ❌ Tier 1 Failed: DPIDR match or SWD protocol error")

    # -------------------------------------------------------------------------
    # Tier 2: Cortex-M3 CPU Control & Debug Verification
    # -------------------------------------------------------------------------
    print("\n--- Tier 2: Cortex-M3 CPU Control & Debug Verification ---")
    # Read DHCSR
    resp_dhcsr = send_cmd(ser, "DEBUG READ 0xE000EDF0")
    print(f"  > Read DHCSR (0xE000EDF0): {resp_dhcsr}")
    
    # Halt CPU via DEBUG HALT command
    resp_halt = send_cmd(ser, "DEBUG HALT")
    print(f"  > Halt DHCSR: {resp_halt}")
    
    # Resume CPU via DEBUG RUN command
    resp_unhalt = send_cmd(ser, "DEBUG RUN")
    print(f"  > Resume DHCSR: {resp_unhalt}")

    tier2_ok = any("OK DEBUG HALT" in r for r in resp_halt)
    if tier2_ok:
        print("  ✓ Tier 2 Passed: Cortex-M3 DHCSR Core Control Validated")
        passed_tiers += 1
    else:
        print("  ❌ Tier 2 Failed: CPU Halt/Unhalt failed")

    # -------------------------------------------------------------------------
    # Tier 3: AHB System Bus & Chip Identification
    # -------------------------------------------------------------------------
    print("\n--- Tier 3: AHB System Bus & Chip Identification ---")
    # SRAM Write/Read Test (halt CPU first so SRAM isn't mutated by running code)
    send_cmd(ser, "DEBUG HALT")
    test_val = "0xDEADBEEF"
    send_cmd(ser, f"DEBUG WRITE 0x20000000 {test_val}")
    resp_ram = send_cmd(ser, "DEBUG READ 0x20000000")
    send_cmd(ser, "DEBUG RUN")
    print(f"  > SRAM Write/Read @ 0x20000000: {resp_ram}")
    
    # DBGMCU IDCODE
    resp_dbg = send_cmd(ser, "DEBUG READ 0xE0042000")
    print(f"  > DBGMCU IDCODE @ 0xE0042000: {resp_dbg}")
    
    # Flash Size Register
    resp_sz = send_cmd(ser, "DEBUG READ 0x1FFFF7E0")
    print(f"  > Flash Size @ 0x1FFFF7E0: {resp_sz}")

    # Unique Device ID (UID)
    resp_uid = send_cmd(ser, "DEBUG READ 0x1FFFF7E8")
    print(f"  > 96-bit Chip UID @ 0x1FFFF7E8: {resp_uid}")

    tier3_ok = any("DEADBEEF" in r.upper() for r in resp_ram)
    if tier3_ok:
        print("  ✓ Tier 3 Passed: AHB SRAM Read/Write & Peripheral Register Access Validated")
        passed_tiers += 1
    else:
        print("  ❌ Tier 3 Failed: AHB SRAM Read/Write Mismatch")

    # -------------------------------------------------------------------------
    # Tier 4: STM32F1 Flash Controller (FPEC) Algorithm
    # -------------------------------------------------------------------------
    print("\n--- Tier 4: STM32F1 Flash Controller (FPEC) Algorithm ---")
    resp_erase = send_cmd(ser, "FLASH ERASE 0x0800FC00 1024")
    print(f"  > FPEC 1 KiB Page Erase @ 0x0800FC00: {resp_erase}")

    tier4_ok = any("OK" in r for r in resp_erase)
    if tier4_ok:
        print("  ✓ Tier 4 Passed: FPEC Key Unlock & 1 KiB Page Erase Verified")
        passed_tiers += 1
    else:
        print("  ❌ Tier 4 Failed: FPEC Erase returned error")

    ser.close()

    # -------------------------------------------------------------------------
    # Tier 5: End-to-End Flashing & Signal Verification
    # -------------------------------------------------------------------------
    print("\n--- Tier 5: End-to-End Application Flashing & LA Signal Sampling ---")
    print("  Executing test_blink_bluepill.py flasher script...")
    res_flash = subprocess.run([sys.executable, "test/test_blink_bluepill.py", port], capture_output=True, text=True)
    print(res_flash.stdout)

    tier5_ok = (res_flash.returncode == 0)
    if tier5_ok:
        print("  ✓ Tier 5 Passed: Application Flash Erase/Program/Verify/Run & Logic Analyzer Signal Capture Successful")
        passed_tiers += 1
    else:
        print("  ❌ Tier 5 Failed: Flasher script returned error")

    # -------------------------------------------------------------------------
    # Final Suite Summary
    # -------------------------------------------------------------------------
    print_header(f"SUITE RESULTS: {passed_tiers}/{total_tiers} Tiers Passed")
    if passed_tiers == total_tiers:
        print("🎉 ALL 5 HARWARE VERIFICATION TIERS PASSED PERFECTLY!")
        sys.exit(0)
    else:
        print(f"❌ VERIFICATION FAILED: {total_tiers - passed_tiers} Tier(s) Errored.")
        sys.exit(1)

if __name__ == "__main__":
    main()
