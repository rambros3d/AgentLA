#!/usr/bin/env python3
"""Map which memory regions are readable via PROGBUF lw on CH32V203.
Distinguishes main-flash-only fault from broad protection."""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
PROGBUF0, PROGBUF1 = 0x20, 0x21
CMD_SET_A1, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x22100A, 0x40000
LW_A0_0_A1, EBREAK = 0x0005A503, 0x00100073
ser = None


def send(cmd, delay=0.04, timeout=1.2):
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
            time.sleep(0.003)
    return [l.strip() for l in buf.decode(errors="replace").splitlines() if l.strip()]


def dmi_w(reg, val):
    send(f"RVSWD WRITE 0x{reg:02X} 0x{val:08X}")


def dmi_r(reg):
    for l in send(f"RVSWD READ 0x{reg:02X}"):
        if "val=" in l:
            return int(l.split("val=")[1].strip(), 16)
    return None


def acs_err():
    for l in send(f"RVSWD READ 0x{ABSTRACTCS:02X}"):
        if "val=" in l:
            return (int(l.split("val=")[1].strip(), 16) >> 8) & 0x7
    return -1


def set_reg(cmd, val):
    dmi_w(DATA0, val)
    dmi_w(COMMAND, cmd)


def mem_read(addr):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, LW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    dmi_w(COMMAND, CMD_POSTEXEC)
    e = acs_err()
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(COMMAND, CMD_GET_A0)
    dmi_w(ABSTRACTCS, 0x700)
    val = dmi_r(DATA0)
    return e, val


def main():
    global ser
    ser = serial.Serial(PORT, BAUD := 115200, timeout=0.5)
    time.sleep(0.5)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l, flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    regions = [
        ("FLASH start       ", 0x08000000),
        ("FLASH mid         ", 0x08010000),
        ("FLASH end         ", 0x080FFC00),
        ("Bootloader        ", 0x1FFF8000),
        ("ESIG chip_id      ", 0x1FFFF704),
        ("Option bytes      ", 0x1FFFF800),
        ("SRAM              ", 0x20000000),
        ("FLASH_CTLR (peri)", 0x40022010),
        ("FLASH_KEYR (peri)", 0x40022004),
        ("DMA (peri)        ", 0x40022000),
    ]
    print("\n-- region sweep --", flush=True)
    for label, addr in regions:
        e, val = mem_read(addr)
        print(f"  {label} 0x{addr:08X}  cmderr={e}  value=0x{val:08X}"
              if val is not None else f"  {label} 0x{addr:08X}  cmderr={e}  value=READFAIL",
              flush=True)

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())