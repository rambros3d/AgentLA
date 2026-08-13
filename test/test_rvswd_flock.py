#!/usr/bin/env python3
"""Test: does MODEKEYR (FLOCK/bit15) unlock fix the cmderr=3 flash read
exception on CH32V203? Full minichlink unlock = KEYR + MODEKEYR."""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
PROGBUF0, PROGBUF1 = 0x20, 0x21
CMD_SET_A1, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x22100A, 0x40000
LW_A0_0_A1, EBREAK = 0x0005A503, 0x00100073
SW_A0_0_A1 = 0x00A5A023
CMD_SET_A0 = 0x23100A
KEYR, MODEKEYR = 0x40022004, 0x40022024
CTLR, STATR = 0x40022010, 0x4002200C
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


def mem_read(addr, label):
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
    print(f"  {label:28s} cmderr={e}  value=0x{val:08X}" if val is not None
          else f"  {label:28s} cmderr={e}  value=READFAIL", flush=True)


def mem_write(addr, val, label):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, SW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    set_reg(CMD_SET_A0, val)
    dmi_w(COMMAND, CMD_POSTEXEC)
    e = acs_err()
    print(f"  {label:28s} cmderr={e}", flush=True)


def main():
    global ser
    print("=" * 64, flush=True)
    print("  FLOCK (MODEKEYR) unlock test", flush=True)
    ser = serial.Serial(PORT, BAUD, timeout=0.5)
    time.sleep(0.5)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l, flush=True)

    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    print("\n-- initial --", flush=True)
    mem_read(CTLR, "FLASH_CTLR")
    mem_read(0x08000000, "FLASH 0x08000000")

    print("\n-- MODEKEYR unlock (FLOCK) --", flush=True)
    mem_write(MODEKEYR, 0x45670123, "MODEKEYR key1")
    mem_write(MODEKEYR, 0xCDEF89AB, "MODEKEYR key2")
    mem_read(CTLR, "FLASH_CTLR")

    print("\n-- after MODEKEYR --", flush=True)
    mem_read(0x08000000, "FLASH 0x08000000 (erased)")
    mem_read(0x08000400, "FLASH 0x08000400 (firmware)")
    mem_read(0x20000000, "SRAM 0x20000000 control")

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())