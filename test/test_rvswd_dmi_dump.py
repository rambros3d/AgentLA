#!/usr/bin/env python3
"""
CH32V203 RVSWD Step-by-Step DMI / Abstract-Command Dump
========================================================
Drives the raw DMI choreography through the probe's dumb RVSWD READ/WRITE
console commands, one register at a time, dumping ABSTRACTCS after every
step. Purpose: isolate which abstract-command step faults when reading
flash (0x08000000+) vs SRAM (0x20000000) vs ESIG (0x1FFFF704).

Sequence reproduced from riscv_dmi.c:riscv_read_mem() (PROGBUF fallback):
  1. halt                : WRITE DMCONTROL=0x80000001, poll DMSTATUS.ALLHALTED
  2. clear errors        : WRITE ABSTRACTCS=0x700
  3. load PROGBUF0/1     : lw a0,0(a1); ebreak
  4. set a1=addr         : WRITE DATA0=addr; COMMAND=0x23100B (set x11)
  5. POSTEXEC            : COMMAND=0x40000  -> runs lw a0,0(a1); ebreak
  6. read a0             : COMMAND=0x22100A (get x10)
  7. read DATA0
After every COMMAND write we read ABSTRACTCS back and report
busy (bit 12), cmderr (bits 10:8), the unknown bit1 (0x02), and progbufsize.
"""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

# --- DMI registers (riscv_dmi.h) ---
DATA0 = 0x04
DMCONTROL = 0x10
DMSTATUS = 0x11
ABSTRACTCS = 0x16
COMMAND = 0x17
ABSTRACTAUTO = 0x18
PROGBUF0 = 0x20
PROGBUF1 = 0x21
SBCS = 0x38

# --- COMMAND encodings ---
CMD_SET_A1 = 0x23100B      # ACCESS_REG | TRANSFER | WRITE | AARSIZE_32 | x11
CMD_GET_A0 = 0x22100A      # ACCESS_REG | TRANSFER | AARSIZE_32 | x10
CMD_POSTEXEC = 0x40000     # ACCESS_REG | POSTEXEC

# --- PROGBUF instructions ---
LW_A0_0_A1 = 0x0005A503
EBREAK = 0x00100073

# --- probe addresses: SRAM control, flash (erased page), flash (untouched), ESIG ---
ADDRS = [
    ("SRAM   0x20000000", 0x20000000),
    ("SRAM   0x20000100", 0x20000100),
    ("ESIG   0x1FFFF704", 0x1FFFF704),
    ("FLASH  0x08000000", 0x08000000),   # erased page (expect 0xFFFFFFFF)
    ("FLASH  0x08000400", 0x08000400),   # untouched firmware page
    ("FLASH  0x08008000", 0x08008000),   # untouched firmware page
]

last_lines = []


def send(cmd, delay=0.05, timeout=1.5):
    """Write a command and read back all response lines.

    Bulk-reads whatever is in the RX buffer until the deadline instead of
    calling ser.readline() (which stalls ~1s per line on a partial line).
    """
    ser.reset_input_buffer()
    ser.write((cmd.strip() + "\r\n").encode())
    time.sleep(delay)
    buf = b""
    deadline = time.time() + timeout
    while time.time() < deadline:
        n = ser.in_waiting
        if n:
            buf += ser.read(n)
        else:
            time.sleep(0.005)
    text = buf.decode(errors="replace")
    return [l.strip() for l in text.splitlines() if l.strip()]


def dmi_w(reg, val):
    lines = send(f"RVSWD WRITE 0x{reg:02X} 0x{val:08X}")
    for l in lines:
        if "ERR" in l:
            print(f"    !! WRITE 0x{reg:02X} failed: {l}")
    return lines


def dmi_r(reg):
    lines = send(f"RVSWD READ 0x{reg:02X}")
    val = None
    for l in lines:
        if "val=" in l:
            val = int(l.split("val=")[1].strip(), 16)
    if val is None:
        print(f"    !! READ 0x{reg:02X} returned nothing: {lines}")
    return val


def acs():
    """Read ABSTRACTCS and format a compact status string."""
    a = dmi_r(ABSTRACTCS)
    if a is None:
        return "ACS=??"
    busy = (a >> 12) & 1
    cmderr = (a >> 8) & 0x7
    bit1 = (a >> 1) & 1
    progbuf = (a >> 24) & 0x1F
    s = f"ACS=0x{a:08X} busy={busy} cmderr={cmderr} bit1={bit1} progbuf={progbuf}"
    return s


def halt():
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        st = dmi_r(DMSTATUS)
        if st is not None and (st & (1 << 9)):
            print(f"  halted OK  (DMSTATUS=0x{st:08X})")
            return True
        time.sleep(0.005)
    st = dmi_r(DMSTATUS)
    print(f"  !! halt timeout  (DMSTATUS=0x{st:08X})")
    return False


def resume():
    dmi_w(DMCONTROL, 0x40000001)
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x00000001)


def read_word(addr, label):
    print(f"\n  --- {label} 0x{addr:08X} ---")
    steps = []

    dmi_w(ABSTRACTCS, 0x700)                 # clear error bits
    steps.append(("clear errors", acs()))

    dmi_w(PROGBUF0, LW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    steps.append(("load progbuf", acs()))

    dmi_w(DATA0, addr)
    dmi_w(COMMAND, CMD_SET_A1)
    steps.append(("set a1", acs()))

    dmi_w(COMMAND, CMD_POSTEXEC)             # <-- suspect step
    steps.append(("POSTEXEC lw", acs()))

    dmi_w(COMMAND, CMD_GET_A0)
    steps.append(("read a0", acs()))

    val = dmi_r(DATA0)
    steps.append(("read DATA0", acs()))

    for name, s in steps:
        print(f"    {name:16s} {s}")
    print(f"    RESULT          DATA0=0x{val:08X}" if val is not None
          else "    RESULT          DATA0=FAILED")
    return val


def main():
    print("=" * 68)
    print("  CH32V203 RVSWD Step-by-Step DMI Dump")
    print("=" * 68)
    print(f"Port: {PORT}")

    global ser
    try:
        ser = serial.Serial(PORT, BAUD, timeout=1.0)
        time.sleep(1.0)
        ser.reset_input_buffer()
    except Exception as e:
        print(f"Cannot open {PORT}: {e}")
        return 1

    print("\n[1] Detect target")
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print(f"    {l}")

    print("\n[2] Read SBCS (confirm no SBA on WCH)")
    sbcs = dmi_r(SBCS)
    print(f"    SBCS=0x{sbcs:08X}" if sbcs is not None else "    SBCS read failed")

    print("\n[3] Halt")
    halted = halt()
    if not halted:
        print("Aborting - could not halt core")
        ser.close()
        return 1

    results = []
    try:
        for label, addr in ADDRS:
            val = read_word(addr, label)
            results.append((label, addr, val))
    finally:
        print("\n[4] Resume")
        resume()

    print("\n" + "=" * 68)
    print("SUMMARY")
    print("=" * 68)
    for label, addr, val in results:
        if val is None:
            print(f"  {label:20s} = READ FAILED")
        elif 0x08000000 <= addr < 0x08010000:
            expect = "erased=>expect 0xFFFFFFFF" if addr == 0x08000000 else "untouched (nonzero)"
            print(f"  {label:20s} = 0x{val:08X}  [{expect}]")
        else:
            print(f"  {label:20s} = 0x{val:08X}")

    ser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())