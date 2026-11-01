#!/usr/bin/env python3
"""
flash_pico2_blink.py
Automated script to flash and verify the Blink test firmware on Raspberry Pi Pico 2 (RP2350)
using the AgentLA CMSIS-DAP probe (VID:PID 1209:414d).
"""

import sys
import subprocess
import time
from pathlib import Path

PROBE_VID_PID = "1209:414d"
CHIP = "RP235x"
ELF_PATH = Path("/run/media/sun/halo/bareBoot/test-firmware/blink-pico2/build/blink_pico2.elf")
BIN_PATH = Path("/run/media/sun/halo/bareBoot/test-firmware/blink-pico2/build/blink_pico2.bin")
PROBE_RS = "/home/sun/sandbox/fedora/.cargo/bin/probe-rs"

def run_cmd(cmd):
    print(f"[CMD] {' '.join(cmd)}")
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
    print(res.stdout)
    return res.returncode == 0

def main():
    print("=" * 65)
    print("AgentLA -> Raspberry Pi Pico 2 (RP2350) Flasher & Verifier")
    print("=" * 65)

    if not ELF_PATH.exists():
        print(f"Error: ELF file not found at {ELF_PATH}")
        sys.exit(1)

    print(f"Firmware ELF: {ELF_PATH} ({ELF_PATH.stat().st_size} bytes)")
    print(f"Target Chip:  {CHIP}")
    print(f"Debug Probe:  AgentLA CMSIS-DAP ({PROBE_VID_PID})")
    print("-" * 65)

    # 1. Probe info
    print("\n[Step 1/3] Detecting Raspberry Pi Pico 2 via AgentLA CMSIS-DAP...")
    info_cmd = [PROBE_RS, "info", "--probe", PROBE_VID_PID, "--chip", CHIP]
    if not run_cmd(info_cmd):
        print("\n[FAIL] Could not connect to Raspberry Pi Pico 2.")
        print("Please verify wiring:")
        print("  AgentLA Pin 6 (GPIO 4 / SWCLK) -> Pico 2 SWCLK (Debug pin 1)")
        print("  AgentLA Pin 7 (GPIO 5 / SWDIO) -> Pico 2 SWDIO (Debug pin 3)")
        print("  AgentLA Pin 3/8/38 (GND)       -> Pico 2 GND   (Debug pin 2 / Pin 28)")
        print("  AgentLA Pin 9 (GPIO 6 / NRST)  -> Pico 2 RUN   (Pin 30) [optional]")
        print("  Pico 2 Power                   -> Plugged into USB-C / VSYS")
        sys.exit(1)

    # 2. Flash and Verify
    print("\n[Step 2/3] Flashing and verifying blink firmware...")
    flash_cmd = [
        PROBE_RS, "download",
        "--probe", PROBE_VID_PID,
        "--chip", CHIP,
        "--verify",
        str(ELF_PATH)
    ]
    if not run_cmd(flash_cmd):
        print("\n[FAIL] Flashing or verification failed.")
        sys.exit(1)

    # 3. Reset and Run
    print("\n[Step 3/3] Resetting Pico 2 target to execute firmware...")
    reset_cmd = [PROBE_RS, "reset", "--probe", PROBE_VID_PID, "--chip", CHIP]
    run_cmd(reset_cmd)

    print("\n" + "=" * 65)
    print("SUCCESS: Raspberry Pi Pico 2 flashed and verified successfully!")
    print("The onboard LED (GPIO 25) should now be blinking at 2 Hz.")
    print("=" * 65)

if __name__ == "__main__":
    main()
