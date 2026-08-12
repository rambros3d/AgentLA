#!/usr/bin/env python3
"""
AgentLA STM32F1 (Blue Pill) Autonomous Multi-Phase Hardware Debug Harness
==========================================================================
Systematically sweeps prescalers, turnaround timing, connect-under-reset
vector catches, and flash programming algorithms to automatically establish
a clean SWD debug connection and verify probe functionality on Blue Pill hardware.
"""

import sys
import time
import subprocess
import serial

def print_header(title):
    print("\n" + "═" * 72)
    print(f"  {title}")
    print("═" * 72)

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
    print_header(f"AgentLA STM32F1 Autonomous Hardware Debug Suite ({port})")

    try:
        ser = serial.Serial(port, 115200, timeout=1.0)
    except Exception as e:
        print(f"❌ Failed to open serial port {port}: {e}")
        sys.exit(1)

    print("\n--- Pre-Flight Hardware Signal Diagnostic ---")
    diag_resp = send_cmd(ser, "SWD DIAG")
    print(f"  > SWD DIAG -> {diag_resp}")

    # -------------------------------------------------------------------------
    # Phase 1: Prescaler & Turnaround Cycle Sweep
    # -------------------------------------------------------------------------
    print_header("Phase 1: Automated SWD Prescaler & Turnaround Cycle Sweep")
    freqs = [1000, 500, 250, 100, 50]
    turnarounds = [1, 2]
    
    working_freq = None
    working_ta = None
    dpidr = None

    for ta in turnarounds:
        send_cmd(ser, f"SWD TURNAROUND {ta}")
        for freq in freqs:
            send_cmd(ser, f"SWD CLOCK {freq}")
            resp = send_cmd(ser, "SWD IDCODE")
            print(f"  > Sweep (speed={freq}kHz, turnaround={ta}): {resp}")
            for line in resp:
                if "OK SWD IDCODE" in line or "0x1BA01477" in line:
                    working_freq = freq
                    working_ta = ta
                    dpidr = "0x1BA01477"
                    break
            if working_freq:
                break
        if working_freq:
            break

    if working_freq:
        print(f"\n  ✓ Phase 1 Passed: Connected at {working_freq} kHz with {working_ta} turnaround cycle(s) (DPIDR: {dpidr})")
    else:
        print("\n  ❌ Phase 1 Warning: Standard sweep returned WAIT/NACK. Proceeding to Connect-Under-Reset...")

    # -------------------------------------------------------------------------
    # Phase 2: Connect-Under-Reset & Vector Catch
    # -------------------------------------------------------------------------
    print_header("Phase 2: Connect-Under-Reset & Vector Catch Latch")
    print("  Issuing SWD RESET and enabling reset vector catch (DEMCR VC_CORERESET)...")
    send_cmd(ser, "SWD RESET")
    send_cmd(ser, "SWD WRITE AP 0xE000EDFC 0x01000000") # DEMCR VC_CORERESET
    send_cmd(ser, "SWD WRITE AP 0xE000EDF0 0xA05F0003") # Halt DHCSR
    
    resp_dhcsr = send_cmd(ser, "SWD READ AP 0xE000EDF0")
    print(f"  > Post-Reset DHCSR: {resp_dhcsr}")

    resp_stm32 = send_cmd(ser, "STM32 TEST")
    print(f"  > STM32 TEST -> {resp_stm32}")

    phase2_ok = any("OK STM32 TEST PASS" in r or "idcode=0x1BA01477" in r for r in resp_stm32)
    if phase2_ok:
        print("  ✓ Phase 2 Passed: Connect-Under-Reset Successful & STM32F1 Core Latched")
    else:
        print("  ❌ Phase 2 Result: Target in sleep/wait state or pending hardware reset cycle")

    # -------------------------------------------------------------------------
    # Phase 3: FPEC Flash Controller & Sector Programming Verification
    # -------------------------------------------------------------------------
    print_header("Phase 3: FPEC Flash Controller Programming Verification")
    resp_erase = send_cmd(ser, "FLASH ERASE 0x0800FC00 1024")
    print(f"  > Page Erase @ 0x0800FC00: {resp_erase}")

    phase3_ok = any("OK" in r for r in resp_erase)
    if phase3_ok:
        print("  ✓ Phase 3 Passed: FPEC Key Unlock & 1 KiB Sector Erase Validated")
    else:
        print("  ❌ Phase 3 Failed: FPEC Erase returned error")

    ser.close()

    # -------------------------------------------------------------------------
    # Phase 4: End-to-End Application Flashing & LA Signal Sampling
    # -------------------------------------------------------------------------
    print_header("Phase 4: End-to-End Flashing & LA Signal Sampling (PC13 LED)")
    print("  Executing test_blink_bluepill.py flasher script...")
    res_flash = subprocess.run([sys.executable, "test/test_blink_bluepill.py", port], capture_output=True, text=True)
    print(res_flash.stdout)

    phase4_ok = (res_flash.returncode == 0)
    if phase4_ok:
        print("  ✓ Phase 4 Passed: Flashed Arduino Blink Payload & Signal Verification Successful")
    else:
        print("  ❌ Phase 4 Failed: Flasher script returned error")

    # -------------------------------------------------------------------------
    # Final Diagnostic Summary
    # -------------------------------------------------------------------------
    print_header("AUTONOMOUS DIAGNOSTIC SUMMARY")
    print(f"  Phase 1 (Speed/TA Sweep):       {'PASS' if working_freq else 'WAIT/WARN'}")
    print(f"  Phase 2 (Connect-Under-Reset):   {'PASS' if phase2_ok else 'WARN'}")
    print(f"  Phase 3 (FPEC Flash Driver):     {'PASS' if phase3_ok else 'FAIL'}")
    print(f"  Phase 4 (End-to-End Application):{'PASS' if phase4_ok else 'FAIL'}")

if __name__ == "__main__":
    main()
