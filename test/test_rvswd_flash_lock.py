#!/usr/bin/env python3
"""
CH32V203 RVSWD: flash lock / controller-state test (corrected).
Uses PROGBUF load/store (like riscv_read_mem) for ALL memory/peripheral
access - RVSWD READ/WRITE only take DMI register numbers, NOT addresses.

Sequence:
  1. DETECT + halt
  2. PROGBUF-read FLASH_CTLR/STATR (real values)
  3. PROGBUF-read flash 0x08000000 -> expect cmderr=3 (exception)
  4. Full minichlink unlock: KEYR(0x40022004), OBKEYR(0x40022008),
     MODEKEYR(0x40022024) - each with both keys
  5. PROGBUF-read FLASH_CTLR again, then flash -> does cmderr clear?
"""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
PROGBUF0, PROGBUF1 = 0x20, 0x21
CMD_SET_A1, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x22100A, 0x40000
CMD_SET_A0 = 0x23100A          # transfer|write|aarsize|reg x10
LW_A0_0_A1, EBREAK = 0x0005A503, 0x00100073
SW_A0_0_A1 = 0x00A5A023        # sw a0, 0(a1)

KEYR, OBKEYR, MODEKEYR = 0x40022004, 0x40022008, 0x40022024
CTLR, STATR = 0x40022010, 0x4002200C

ser = None


def send(cmd, delay=0.05, timeout=1.5):
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
            v = int(l.split("val=")[1].strip(), 16)
            return (v >> 8) & 0x7
    return -1


def load_prog(p0, p1):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, p0)
    dmi_w(PROGBUF1, p1)


def set_reg(reg_cmd, val):
    dmi_w(DATA0, val)
    dmi_w(COMMAND, reg_cmd)


def mem_read(addr, label):
    """lw a0, 0(a1); ebreak -> DATA0"""
    load_prog(LW_A0_0_A1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    dmi_w(COMMAND, CMD_POSTEXEC)
    e = acs_err()
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(COMMAND, CMD_GET_A0)
    dmi_w(ABSTRACTCS, 0x700)
    val = dmi_r(DATA0)
    print(f"  {label:30s} cmderr={e}  value=0x{val:08X}" if val is not None
          else f"  {label:30s} cmderr={e}  value=READFAIL")
    return val, e


def mem_write(addr, val, label):
    """sw a0, 0(a1); ebreak"""
    load_prog(SW_A0_0_A1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    set_reg(CMD_SET_A0, val)
    dmi_w(COMMAND, CMD_POSTEXEC)
    e = acs_err()
    print(f"  {label:30s} cmderr={e}")


def main():
    global ser
    print("=" * 68)
    print("  CH32V203 flash lock test (PROGBUF-based, corrected)")
    print("=" * 68)
    ser = serial.Serial(PORT, BAUD, timeout=0.5)
    time.sleep(0.8)
    ser.reset_input_buffer()

    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l)

    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    print("\n-- controller state before --")
    mem_read(CTLR, "FLASH_CTLR")
    mem_read(STATR, "FLASH_STATR")
    mem_read(0x08000000, "FLASH 0x08000000 (erased?)")
    mem_read(0x20000000, "SRAM 0x20000000 (control)")

    print("\n-- full minichlink unlock (KEYR+OBKEYR+MODEKEYR) --")
    mem_write(KEYR, 0x45670123, "KEYR key1")
    mem_write(KEYR, 0xCDEF89AB, "KEYR key2")
    mem_write(OBKEYR, 0x45670123, "OBKEYR key1")
    mem_write(OBKEYR, 0xCDEF89AB, "OBKEYR key2")
    mem_write(MODEKEYR, 0x45670123, "MODEKEYR key1")
    mem_write(MODEKEYR, 0xCDEF89AB, "MODEKEYR key2")

    print("\n-- controller state after unlock --")
    mem_read(CTLR, "FLASH_CTLR")
    mem_read(STATR, "FLASH_STATR")
    mem_read(0x08000000, "FLASH 0x08000000 (erased?)")
    mem_read(0x08000400, "FLASH 0x08000400 (firmware)")
    mem_read(0x20000000, "SRAM 0x20000000 (control)")

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())