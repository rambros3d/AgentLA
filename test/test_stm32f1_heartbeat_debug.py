#!/usr/bin/env python3
"""
AgentLA STM32F1 (Blue Pill) Heartbeat + Hardware Debug Verification Suite
=========================================================================
5-stage automated verification of the double-pulse heartbeat firmware and
interactive Cortex-M3 hardware debugging capabilities of the AgentLA RP2350
probe, all driven over the CDC serial command interface.

Stage 1: Build the heartbeat firmware via PlatformIO.
Stage 2: Program flash via SWD FPEC controller (erase/write/verify).
Stage 3: Interactive debugging: HALT, single-STEP, register READ, memory
         READ/WRITE mutation on the running heartbeat target.
Stage 4: Resume (RUN) and verify the PC13 LED double-pulse signal on GPIO22
         (L_CH2) using the AgentLA 100 MHz logic analyzer.
Stage 5: Summary.

Wiring:
  AgentLA GPIO4  (SWCLK) -> BluePill SWCLK
  AgentLA GPIO5  (SWDIO) <-> BluePill SWDIO
  AgentLA GPIO6  (nRST)  -> BluePill nRST
  AgentLA GND             -> BluePill GND
  AgentLA GPIO22 (L_CH2)  -> BluePill PC13   (LED observation)

BluePill must be externally powered (3.3 V or 5 V USB).

Usage:
  python3 test/test_stm32f1_heartbeat_debug.py            # auto-detect port
  python3 test/test_stm32f1_heartbeat_debug.py /dev/ttyACM0
  python3 test/test_stm32f1_heartbeat_debug.py --no-build # skip PlatformIO
"""

import glob
import struct
import subprocess
import sys
import time
from pathlib import Path

import serial


PROJECT_DIR = Path(__file__).resolve().parent / "bluepill_blink"
FIRMWARE_BIN = PROJECT_DIR / ".pio" / "build" / "bluepill_f103c8" / "firmware.bin"

LED_GPIO = 22          # AgentLA capture channel wired to BluePill PC13
FLASH_BASE = 0x08000000
WRITE_CHUNK = 256      # firmware SWD flash write chunk limit

# Heartbeat timing tolerances (ms) — PC13 is active-LOW.
HEARTBEAT = {"pulse_on": (100, 60, 140),
             "pulse_off": (100, 60, 140),
             "rest": (700, 580, 820)}


def print_header(title):
    print("\n" + "═" * 72)
    print(f"  {title}")
    print("═" * 72)


def build_firmware():
    """Stage 1: compile the heartbeat firmware with PlatformIO."""
    print_header("STAGE 1: Build Heartbeat Firmware (PlatformIO)")
    result = subprocess.run(
        ["pio", "run", "--project-dir", str(PROJECT_DIR)],
        capture_output=True, text=True, timeout=180,
    )
    if result.returncode != 0 or not FIRMWARE_BIN.exists():
        print("  ✗ Build failed:")
        print(result.stdout)
        print(result.stderr)
        sys.exit(1)
    size = FIRMWARE_BIN.stat().st_size
    print(f"  ✓ Built {FIRMWARE_BIN.name}: {size} bytes (flash {size/1024:.1f} KiB)")
    return size


def find_serial_port():
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
            continue
    return None


def send_cmd(ser, cmd, timeout=2.0):
    """Send a command and collect response lines.

    Returns once the response stream goes idle after a terminal OK/ERR line,
    so a slow command doesn't stall the caller for the full timeout while
    still catching multi-line responses (e.g. DEBUG REGS prints its OK line
    first and the register lines after)."""
    ser.reset_input_buffer()
    ser.write((cmd + '\r\n').encode())
    time.sleep(0.05)
    deadline = time.time() + timeout
    lines = []
    terminal = False
    while time.time() < deadline:
        if ser.in_waiting:
            raw = ser.read(ser.in_waiting).decode(errors='replace')
            for ln in raw.splitlines():
                ln = ln.strip()
                if ln and not ln.startswith('> '):
                    lines.append(ln)
                    if ln.startswith('OK') or ln.startswith('ERR'):
                        terminal = True
        elif terminal:
            # Terminal line seen and no more data in flight: give trailing
            # multi-line responses a short grace window before returning.
            time.sleep(0.1)
            if not ser.in_waiting:
                return lines
        time.sleep(0.02)
    return lines


def cmd_ok(lines, prefix):
    return any(l.startswith(prefix) for l in lines)


def stage_connect(ser, port):
    """Stage 0: SWD connection test to the Blue Pill."""
    print_header(f"CONNECT: SWD Target ({port})")
    lines = send_cmd(ser, 'SWD TEST', timeout=3.0)
    for l in lines:
        print(f"  {l}")
    if not cmd_ok(lines, 'OK SWD TEST'):
        print("  ✗ SWD TEST FAILED")
        print("    Wiring: GPIO4→SWCLK, GPIO5↔SWDIO, GPIO6→nRST, GND→GND")
        sys.exit(1)
    print("  ✓ Blue Pill detected")


def stage_flash(ser):
    """Stage 2: erase + write + verify the heartbeat firmware via SWD FPEC."""
    print_header("STAGE 2: SWD FPEC Flash Programming")
    fw = FIRMWARE_BIN.read_bytes()
    erase_sz = (len(fw) + 0x3FF) & ~0x3FF
    print(f"  Erasing 0x{erase_sz:X} bytes at 0x{FLASH_BASE:08X}...")
    lines = send_cmd(ser, f'FLASH ERASE 0x{FLASH_BASE:08X} 0x{erase_sz:X}', timeout=5.0)
    for l in lines:
        print(f"  {l}")
    if not cmd_ok(lines, 'OK FLASH ERASE'):
        print("  ✗ Flash erase failed"); sys.exit(1)

    chunks = [fw[i:i + WRITE_CHUNK] for i in range(0, len(fw), WRITE_CHUNK)]
    print(f"  Writing {len(fw)} bytes in {len(chunks)} chunks...")
    for idx, chunk in enumerate(chunks):
        addr = FLASH_BASE + idx * WRITE_CHUNK
        lines = send_cmd(ser, f'FLASH WRITE 0x{addr:08X} {chunk.hex().upper()}', timeout=5.0)
        if not cmd_ok(lines, 'OK FLASH WROTE'):
            print(f"  ✗ Chunk {idx} failed at 0x{addr:08X}")
            for l in lines:
                print(f"    {l}")
            sys.exit(1)

    print("  Verifying...")
    for idx, chunk in enumerate(chunks):
        addr = FLASH_BASE + idx * WRITE_CHUNK
        lines = send_cmd(ser, f'FLASH VERIFY 0x{addr:08X} {chunk.hex().upper()}', timeout=5.0)
        if not cmd_ok(lines, 'OK FLASH VERIFY'):
            print(f"  ✗ Verify failed at 0x{addr:08X}")
            sys.exit(1)
    print(f"  ✓ Programmed + verified {len(fw)} bytes")


def stage_debug(ser):
    """Stage 3: HALT/STEP/REGS/READ/WRITE on the running heartbeat target."""
    print_header("STAGE 3: Interactive Cortex-M3 Hardware Debug")
    # The target was flashed while the OLD firmware was still executing from
    # the same flash, so the CPU has faulted. Pulse nRST (GPIO6) to reboot
    # into the freshly programmed heartbeat firmware before debugging it.
    print("  Rebooting target into freshly flashed firmware (DEBUG RESET)...")
    lines = send_cmd(ser, 'DEBUG RESET', timeout=3.0)
    for l in lines:
        print(f"  {l}")
    if not cmd_ok(lines, 'OK DEBUG RESET'):
        print("  ✗ DEBUG RESET failed"); sys.exit(1)
    # HALT the running firmware.
    lines = send_cmd(ser, 'DEBUG HALT', timeout=3.0)
    for l in lines:
        print(f"  {l}")
    if not cmd_ok(lines, 'OK DEBUG HALT'):
        print("  ✗ DEBUG HALT failed"); sys.exit(1)

    # Single-step the halted CPU.
    lines = send_cmd(ser, 'DEBUG STEP', timeout=3.0)
    for l in lines:
        print(f"  {l}")
    if not cmd_ok(lines, 'OK DEBUG STEP'):
        print("  ✗ DEBUG STEP failed"); sys.exit(1)

    # Read all core registers.
    lines = send_cmd(ser, 'DEBUG REGS', timeout=6.0)
    seen = set()
    for l in lines:
        print(f"  {l}")
        if '=' in l and l.split('=')[0].strip() in {
                'R0', 'R1', 'R2', 'R3', 'R4', 'R5', 'R6', 'R7',
                'R8', 'R9', 'R10', 'R11', 'R12', 'SP', 'LR', 'PC', 'xPSR'}:
            seen.add(l.split('=')[0].strip())
    if not cmd_ok(lines, 'OK DEBUG REGS') or len(seen) < 17:
        print(f"  ✗ DEBUG REGS incomplete ({len(seen)}/17 registers)")
        sys.exit(1)

    # Memory mutation: write a unique marker to a scratch SRAM word, read it
    # back, and confirm the write took effect on the halted target.
    marker_addr = 0x20000000
    marker = (time.time_ns() & 0x7FFFFFFF) | 0x01  # keep bit 9 free of junk
    lines = send_cmd(ser, f'DEBUG WRITE 0x{marker_addr:08X} 0x{marker:08X}', timeout=3.0)
    if not cmd_ok(lines, 'OK DEBUG WRITE'):
        print("  ✗ DEBUG WRITE failed"); sys.exit(1)
    lines = send_cmd(ser, f'DEBUG READ 0x{marker_addr:08X}', timeout=3.0)
    got = None
    for l in lines:
        print(f"  {l}")
        if 'OK DEBUG READ' in l:
            try:
                # Line format: OK DEBUG READ 0x<addr>=0x<value> — take the value
                got = int(l.split('=')[1], 16)
            except Exception:
                got = None
    if got != marker:
        print(f"  ✗ Memory mutation mismatch: wrote 0x{marker:08X} read 0x{got:08X}")
        sys.exit(1)
    print("  ✓ Halt, step, register inspection, and memory mutation valid")


def parse_raw_samples(lines):
    """Extract bit-0 levels (channel 0 = GPIO22) from RAW capture lines."""
    samples = bytearray()
    for ln in lines:
        ln = ln.strip()
        if ln.startswith('RAW '):
            try:
                samples.append(int(ln[4:6], 16))
            except ValueError:
                continue
    # bit0 of each byte is channel 0 (GPIO22). Return 1 = HIGH, 0 = LOW.
    return bytearray(b & 0x01 for b in samples)


def measure_pulse_durations(samples, rate):
    """Turn a level array into (low_ms, high_ms, ...) run durations."""
    if not samples:
        return []
    durations = []
    cur_level = samples[0]
    length = 1
    for s in samples[1:]:
        if s == cur_level:
            length += 1
        else:
            durations.append((cur_level, length * 1000.0 / rate))
            cur_level = s
            length = 1
    durations.append((cur_level, length * 1000.0 / rate))
    return durations


def stage_la(ser):
    """Stage 4: resume and verify the PC13 double-pulse on GPIO22 (L_CH2)."""
    print_header("STAGE 4: Logic Analyzer Double-Pulse Verification (GPIO22)")
    send_cmd(ser, 'DEBUG RUN', timeout=3.0)
    time.sleep(1.0)  # let the CPU resume and settle into the heartbeat

    rate = 100000         # 100 kHz -> 10 samples/ms, ~3.9 s window
    samples = 300000      # 3 seconds of the 1 Hz heartbeat
    lines = send_cmd(ser, f'CONFIGURE channels={LED_GPIO} rate={rate}', timeout=3.0)
    if not cmd_ok(lines, 'OK configured'):
        print("  ✗ CONFIGURE failed:"); [print(f"  {l}") for l in lines]
        sys.exit(1)

    lines = send_cmd(ser, f'CAPTURE samples={samples} mode=RAW', timeout=15.0)
    raw_count = sum(1 for l in lines if l.startswith('RAW '))
    if raw_count < 1000 or not any(l.startswith('DONE') for l in lines):
        print(f"  ✗ Capture insufficient ({raw_count} samples)")
        sys.exit(1)

    levels = parse_raw_samples(lines)
    print(f"  Captured {len(levels)} samples @ {rate/1000:.0f} kHz")
    lows = sum(1 for b in levels if b == 0)
    highs = sum(1 for b in levels if b == 1)
    print(f"  HIGH {highs}  LOW {lows}  transitions detected")

    # Segment the level stream into cycles and score each pulse length.
    durations = measure_pulse_durations(levels, rate)
    # Skip the initial partial pulse; scan for full 1000 ms cycles.
    ok_cycles = 0
    cycles_seen = 0
    i = 0
    # Find a LOW run to anchor the beginning of a beat.
    while i < len(durations) and durations[i][0] != 0:
        i += 1
    while i + 3 < len(durations):
        d0, d1, d2, d3 = durations[i], durations[i + 1], durations[i + 2], durations[i + 3]
        # Expect: LOW(on) HIGH(off) LOW(on) HIGH(rest)
        lo, hi, lo2, hi2 = d0[1], d1[1], d2[1], d3[1]
        lo_ok = HEARTBEAT['pulse_on'][0] - HEARTBEAT['pulse_on'][1] <= lo <= HEARTBEAT['pulse_on'][0] + HEARTBEAT['pulse_on'][2]
        off_ok = HEARTBEAT['pulse_off'][0] - HEARTBEAT['pulse_off'][1] <= hi <= HEARTBEAT['pulse_off'][0] + HEARTBEAT['pulse_off'][2]
        lo2_ok = HEARTBEAT['pulse_on'][0] - HEARTBEAT['pulse_on'][1] <= lo2 <= HEARTBEAT['pulse_on'][0] + HEARTBEAT['pulse_on'][2]
        rest_ok = HEARTBEAT['rest'][0] - HEARTBEAT['rest'][1] <= hi2 <= HEARTBEAT['rest'][0] + HEARTBEAT['rest'][2]
        cycles_seen += 1
        if lo_ok and off_ok and lo2_ok and rest_ok:
            ok_cycles += 1
            print(f"  ✓ Heartbeat: {lo:.0f}ms ON  {hi:.0f}ms OFF  "
                  f"{lo2:.0f}ms ON  {hi2:.0f}ms REST")
        else:
            print(f"  ✗ Miss: {lo:.0f}ms ON  {hi:.0f}ms OFF  "
                  f"{lo2:.0f}ms ON  {hi2:.0f}ms REST")
        i += 4

    if cycles_seen == 0:
        print("  ✗ No heartbeat cycles found in capture")
        sys.exit(1)
    if ok_cycles < max(1, cycles_seen - 1):
        print(f"  ✗ Double-pulse pattern only matched {ok_cycles}/{cycles_seen} cycles")
        sys.exit(1)
    print(f"  ✓ Double-pulse 100/100/100/700 ms pattern verified "
          f"({ok_cycles}/{cycles_seen} cycles)")


def main():
    no_build = '--no-build' in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith('--')]

    if not no_build:
        build_firmware()

    port = args[0] if args else None
    if not port:
        print_header("PORT DETECTION")
        print("  Auto-detecting AgentLA...")
        port = find_serial_port()
        if not port:
            print("  ✗ AgentLA not found. Check USB / udev.")
            sys.exit(1)
        print(f"  ✓ {port}")

    ser = serial.Serial(port, 115200, timeout=0.5)
    # Drain any leftover bulk text in OS/USB serial buffer
    deadline = time.time() + 2.0
    while time.time() < deadline:
        if ser.in_waiting:
            ser.read(ser.in_waiting)
            deadline = time.time() + 0.5
        time.sleep(0.05)
    ser.reset_input_buffer()

    stage_connect(ser, port)
    stage_flash(ser)
    stage_debug(ser)
    stage_la(ser)
    ser.close()

    print_header("STAGE 5: SUMMARY")
    print("  ✓ Heartbeat firmware built, flashed, debugged interactively,")
    print("  ✓ CPU halts/steps/registers/memory validated over CDC")
    print("  ✓ PC13 double-pulse signal verified on GPIO22 logic analyzer")
    print("  ALL HEARTBEAT + DEBUG STAGES PASSED!")
    sys.exit(0)


if __name__ == '__main__':
    main()