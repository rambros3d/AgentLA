#!/usr/bin/env python3
"""Flash a 1-second blink firmware onto a BluePill STM32F103 via AgentLA SWD.

The blink firmware is an Arduino sketch built with PlatformIO.
It runs on the STM32F103 and toggles PC13 (active-LOW LED) at ~1 Hz.

Wiring:
  AgentLA GPIO4 (SWCLK) -> BluePill SWCLK
  AgentLA GPIO5 (SWDIO) <-> BluePill SWDIO
  AgentLA GND            -> BluePill GND

BluePill must be externally powered (3.3V or 5V USB).

Usage:
  python3 test_blink_bluepill.py              # auto-detect serial port
  python3 test_blink_bluepill.py /dev/ttyACM0  # explicit port
  python3 test_blink_bluepill.py --no-build   # skip build, use existing binary
"""
import serial
import subprocess
import time
import sys
import glob
import os
from pathlib import Path


# ── Build ──────────────────────────────────────────────────────────────

PROJECT_DIR = Path(__file__).parent / "bluepill_blink"
FIRMWARE_BIN = PROJECT_DIR / ".pio" / "build" / "bluepill_f103c8" / "firmware.bin"


def build_firmware():
    """Build the blink firmware using PlatformIO."""
    print(f"  Source: {PROJECT_DIR}")
    result = subprocess.run(
        ["pio", "run", "--project-dir", str(PROJECT_DIR)],
        capture_output=True,
        text=True,
        timeout=120,
    )
    if result.returncode != 0:
        print("  ✗ Build failed:")
        print(result.stdout)
        print(result.stderr)
        sys.exit(1)

    if not FIRMWARE_BIN.exists():
        print(f"  ✗ Binary not found at {FIRMWARE_BIN}")
        sys.exit(1)

    size = FIRMWARE_BIN.stat().st_size
    print(f"  ✓ Built: {size} bytes")
    return size


def load_firmware():
    """Load the pre-built binary from disk."""
    if not FIRMWARE_BIN.exists():
        print(f"  ✗ Binary not found at {FIRMWARE_BIN}")
        print("    Run without --no-build to compile first.")
        sys.exit(1)
    size = FIRMWARE_BIN.stat().st_size
    print(f"  ✓ Loaded: {size} bytes")
    return size


# ── Serial helpers ─────────────────────────────────────────────────────

def find_serial_port():
    """Auto-detect the AgentLA serial port."""
    candidates = sorted(glob.glob('/dev/ttyACM*') + glob.glob('/dev/ttyUSB*'))
    for port in candidates:
        try:
            ser = serial.Serial(port, 115200, timeout=2)
            time.sleep(0.5)
            ser.reset_input_buffer()
            ser.write(b'IDENTIFY\r\n')
            time.sleep(1.0)
            resp = ser.read(ser.in_waiting or 1024).decode(errors='replace')
            ser.close()
            if 'AGENTLA' in resp:
                return port
        except Exception:
            pass
    return None


def send_cmd(ser, cmd, timeout=2.0):
    """Send a command string and collect response lines."""
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    time.sleep(0.3)
    deadline = time.time() + timeout
    lines = []
    while time.time() < deadline:
        if ser.in_waiting:
            raw = ser.read(ser.in_waiting).decode(errors='replace')
            for ln in raw.splitlines():
                ln = ln.strip()
                if ln and not ln.startswith('> '):
                    lines.append(ln)
        time.sleep(0.05)
    return lines


def cmd_ok(lines, prefix='OK'):
    return any(l.startswith(prefix) for l in lines)


# ── Main ───────────────────────────────────────────────────────────────

def main():
    print('╔══════════════════════════════════════════════════╗')
    print('║  BluePill STM32F103 Blink Flasher  (via SWD)    ║')
    print('║  Arduino framework via PlatformIO               ║')
    print('╚══════════════════════════════════════════════════╝')

    no_build = '--no-build' in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith('--')]

    # 1. Build or load firmware
    print('\n[1/6] Firmware...')
    if no_build:
        fw_size = load_firmware()
    else:
        fw_size = build_firmware()

    # 2. Find serial port
    port = args[0] if args else None
    if not port:
        print('\n[2/6] Auto-detecting AgentLA...')
        port = find_serial_port()
        if not port:
            print('  ✗ AgentLA not found.  Check USB connection.')
            print('    Usage: python3 test_blink_bluepill.py [/dev/ttyACMx]')
            sys.exit(1)
    print(f'  ✓ Port: {port}')

    # 3. Connect
    ser = serial.Serial(port, 115200, timeout=2)
    time.sleep(1.0)

    # 4. SWD test
    print('\n[3/6] SWD connection test...')
    lines = send_cmd(ser, 'SWD TEST', timeout=3.0)
    for l in lines:
        print(f'  {l}')
    if not cmd_ok(lines, 'OK SWD TEST'):
        print('\n  ✗ SWD TEST FAILED')
        print('    Wiring:  GPIO4→SWCLK,  GPIO5↔SWDIO,  GND→GND')
        print('    Make sure BluePill is powered (3.3 V or 5 V USB).')
        ser.close()
        sys.exit(1)
    print('  ✓ Target detected')

    # 5. Erase flash (1 KB minimum for STM32F1)
    erase_sz = max(0x400, fw_size)
    # Round up to 1 KB boundary
    erase_sz = (erase_sz + 0x3FF) & ~0x3FF
    print(f'\n[4/6] Erasing flash at 0x08000000 ({erase_sz} bytes)...')
    lines = send_cmd(ser, f'FLASH ERASE 0x08000000 0x{erase_sz:X}', timeout=5.0)
    for l in lines:
        print(f'  {l}')
    if not cmd_ok(lines, 'OK FLASH ERASE'):
        print('  ✗ Flash erase failed'); ser.close(); sys.exit(1)
    print('  ✓ Erased')

    # 6. Write firmware (chunked — firmware limits to 256 bytes per write)
    fw_data = FIRMWARE_BIN.read_bytes()
    CHUNK = 256  # max bytes per FLASH WRITE command
    chunks = [fw_data[i:i+CHUNK] for i in range(0, len(fw_data), CHUNK)]
    print(f'\n[5/6] Writing firmware ({len(fw_data)} bytes, {len(chunks)} chunks)...')
    for idx, chunk in enumerate(chunks):
        addr = 0x08000000 + idx * CHUNK
        hex_chunk = chunk.hex().upper()
        lines = send_cmd(ser, f'FLASH WRITE 0x{addr:08X} {hex_chunk}', timeout=5.0)
        if not cmd_ok(lines, 'OK FLASH WROTE'):
            print(f'  ✗ Chunk {idx} failed at 0x{addr:08X}')
            for l in lines:
                print(f'    {l}')
            ser.close()
            sys.exit(1)
    print(f'  ✓ Written {len(fw_data)} bytes')

    # 7. Verify (chunked)
    print('\n[6/6] Verifying flash...')
    for idx, chunk in enumerate(chunks):
        addr = 0x08000000 + idx * CHUNK
        hex_chunk = chunk.hex().upper()
        lines = send_cmd(ser, f'FLASH VERIFY 0x{addr:08X} {hex_chunk}', timeout=5.0)
        if not cmd_ok(lines, 'OK FLASH VERIFY'):
            print(f'  ✗ Verify failed at 0x{addr:08X}')
            for l in lines:
                print(f'    {l}')
            ser.close()
            sys.exit(1)
    print('  ✓ Verified')

    # 8. Reset target via AIRCR system reset
    print('\n  Resetting target...')
    send_cmd(ser, 'SWD WRITE AP 0x00 0x23000012', timeout=1.0)  # CSW = 32-bit
    send_cmd(ser, 'SWD WRITE AP 0x04 0xE000ED0C', timeout=1.0)  # TAR = AIRCR
    send_cmd(ser, 'SWD WRITE AP 0x0C 0x05FA0004', timeout=1.0)  # DRW = SYSRESETREQ
    time.sleep(0.5)

    ser.close()

    print('\n╔══════════════════════════════════════════════════╗')
    print('║  ✓ Flash complete!                               ║')
    print('║  BluePill PC13 LED should blink at ~1 Hz.        ║')
    print('║  (LED active-LOW: ON when pin = 0)               ║')
    print('╚══════════════════════════════════════════════════╝')


if __name__ == '__main__':
    main()
