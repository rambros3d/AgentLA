#!/usr/bin/env python3
"""Unprotect CH32V203, retry 3: mirrors minichlink's SetRAMSplit option-byte
sequence exactly (RDPR first, re-unlock OBKEYR before later writes), restores
all option bytes to factory values, no pre-mass-erase (chip already erased).
Run AFTER a clean power cycle of the target board."""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
FLASH_DELAY = 0.15

DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
PROGBUF0, PROGBUF1 = 0x20, 0x21
CMD_SET_A1, CMD_SET_A0, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x23100A, 0x22100A, 0x40000
SW_A0_0_A1, SH_A0_0_A1, LW_A0_0_A1, EBREAK = 0x00A5A023, 0x00A59023, 0x0005A503, 0x00100073

KEYR, OBKEYR, MODEKEYR = 0x40022004, 0x40022008, 0x40022024
CTLR, STATR = 0x40022010, 0x4002200C
MER, OPTER, OPTPG, STRT = 0x4, 0x20, 0x10, 0x40
KEY1, KEY2 = 0x45670123, 0xCDEF89AB
OB_BASE = 0x1FFFF800
ser = None


def send(cmd, delay=0.06, timeout=2.0):
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
            time.sleep(0.004)
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


def mem_write32(addr, val):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, SW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    set_reg(CMD_SET_A0, val)
    dmi_w(COMMAND, CMD_POSTEXEC)
    return acs_err()


def mem_write16(addr, val):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, SH_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    set_reg(CMD_SET_A0, val)
    dmi_w(COMMAND, CMD_POSTEXEC)
    return acs_err()


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
    return e, dmi_r(DATA0)


def reg(name, addr):
    e, v = mem_read(addr)
    print(f"    {name:14s} cmderr={e} value=0x{v:08X}", flush=True)
    return v


def wait_flash(label, timeout_ms=30000):
    deadline = time.time() + timeout_ms / 1000.0
    while time.time() < deadline:
        e, st = mem_read(STATR)
        if e != 0:
            time.sleep(0.2)
            continue
        if (st & 3) == 0:
            print(f"    wait_flash: {label} idle (STATR=0x{st:08X})", flush=True)
            return True
        time.sleep(0.2)
    e, st = mem_read(STATR)
    print(f"    !! wait_flash: {label} TIMEOUT (STATR=0x{st:08X})", flush=True)
    return False


def unlock():
    print("  -- unlock (KEYR, OBKEYR, MODEKEYR) --", flush=True)
    for name, reg_addr in (("KEYR", KEYR), ("OBKEYR", OBKEYR), ("MODEKEYR", MODEKEYR)):
        for key in (KEY1, KEY2):
            e = mem_write32(reg_addr, key)
            if e != 0:
                print(f"    !! {name} key cmderr={e}", flush=True)
            time.sleep(FLASH_DELAY)
    reg("CTLR", CTLR)


def main():
    global ser
    print("=" * 64, flush=True)
    print("  CH32V203 UNPROTECT v3 (mirror minichlink option-byte sequence)", flush=True)
    ser = serial.Serial(PORT, 115200, timeout=0.5)
    time.sleep(0.5)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.8):
        print("  " + l, flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    print("\n-- state after power cycle --", flush=True)
    obr = reg("OBR", 0x4002201C)
    for off in (0x00, 0x04, 0x08, 0x0C):
        reg(f"OB[{off:02X}]", OB_BASE + off)
    if (obr & 2) == 0:
        print("  !! chip is NOT read protected; nothing to do", flush=True)
        ser.close()
        return 0

    unlock()

    print("\n-- option byte erase --", flush=True)
    for key in (KEY1, KEY2):
        mem_write32(OBKEYR, key); time.sleep(FLASH_DELAY)
    mem_write32(CTLR, OPTER); time.sleep(FLASH_DELAY)
    mem_write32(CTLR, OPTER | STRT); time.sleep(FLASH_DELAY)
    if not wait_flash("option byte erase"):
        ser.close()
        return 1
    reg("CTLR", CTLR)

    print("\n-- program option bytes (RDPR first, minichlink order) --", flush=True)
    # RDPR/nRDPR (triggers auto mass-erase on regression), then USER/nUSER,
    # then WRPR0-3 and DATA0/1 restored to factory values.
    steps = [
        ("RDPR ", 0x00, 0x5AA5),
        ("USER ", 0x02, 0x3FC0),
        ("WRPR0", 0x04, 0x00FF),
        ("WRPR1", 0x06, 0x00FF),
        ("WRPR2", 0x08, 0x00FF),
        ("WRPR3", 0x0A, 0x00FF),
        ("DATA0", 0x0C, 0x00FF),
        ("DATA1", 0x0E, 0x00FF),
    ]
    for i, (name, off, val) in enumerate(steps):
        print(f"    -> {name} OB+{off:02X} = 0x{val:04X}", flush=True)
        if i >= 1:
            for key in (KEY1, KEY2):
                mem_write32(OBKEYR, key); time.sleep(FLASH_DELAY)
            wait_flash("obkeyr unlock")
        e = mem_write32(CTLR, OPTPG)
        time.sleep(FLASH_DELAY)
        e = mem_write16(OB_BASE + off, val)
        time.sleep(FLASH_DELAY)
        print(f"       cmderr={e}", flush=True)
        if not wait_flash(f"write {name}"):
            ser.close()
            return 1
    time.sleep(0.5)
    mem_write32(CTLR, 0); time.sleep(FLASH_DELAY)
    for off in (0x00, 0x04, 0x08, 0x0C):
        reg(f"OB[{off:02X}]", OB_BASE + off)
    reg("CTLR", CTLR)

    print("\n-- reset to reload option bytes --", flush=True)
    dmi_w(DMCONTROL, 0x40000001)
    time.sleep(0.2)
    dmi_w(DMCONTROL, 0x00000001)
    time.sleep(0.5)
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.8):
        print("  " + l, flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    print("\n-- verify unprotect --", flush=True)
    obr = reg("OBR", 0x4002201C)
    for off in (0x00, 0x04, 0x08, 0x0C):
        reg(f"OB[{off:02X}]", OB_BASE + off)
    e, f0 = mem_read(0x08000000)
    e2, f1 = mem_read(0x08000400)
    print(f"    FLASH[0x08000000] cmderr={e} = 0x{f0:08X}", flush=True)
    print(f"    FLASH[0x08000400] cmderr={e2} = 0x{f1:08X}", flush=True)
    ok = (obr & 2) == 0 and e == 0 and e2 == 0
    print(("    UNPROTECT OK — flash readable" if ok else "    !! unexpected result"), flush=True)

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())