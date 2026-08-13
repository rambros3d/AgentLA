#!/usr/bin/env python3
"""Unprotect CH32V203: clear read protection by programming RDPR=0xA5
and mass-erasing. Sequence derived from minichlink + WCH EVT optionbytes
example. Destructive: erases the entire chip."""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"

DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
PROGBUF0, PROGBUF1 = 0x20, 0x21
CMD_SET_A1, CMD_SET_A0, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x23100A, 0x22100A, 0x40000
SW_A0_0_A1, SH_A0_0_A1, LW_A0_0_A1, EBREAK = 0x00A5A023, 0x00A59023, 0x0005A503, 0x00100073

KEYR, OBKEYR, MODEKEYR = 0x40022004, 0x40022008, 0x40022024
CTLR, STATR = 0x40022010, 0x4002200C
MER, OPTER, OPTPG, STRT = 0x4, 0x20, 0x10, 0x40
KEY1, KEY2 = 0x45670123, 0xCDEF89AB
RDPR_ADDR = 0x1FFFF800
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


def wait_flash(label, timeout_ms=8000):
    deadline = time.time() + timeout_ms / 1000.0
    while time.time() < deadline:
        e, st = mem_read(STATR)
        if e != 0:
            print(f"  !! {label}: cmderr={e} reading STATR", flush=True)
            return False
        if (st & 3) == 0:
            if st & 0x10:
                print(f"  !! {label}: WRPRTERR set (STATR=0x{st:08X})", flush=True)
            return True
        time.sleep(0.05)
    e, st = mem_read(STATR)
    print(f"  !! {label}: TIMEOUT (STATR=0x{st:08X})", flush=True)
    return False


def unlock():
    for reg in (KEYR, OBKEYR, MODEKEYR):
        if mem_write32(reg, KEY1) != 0 or mem_write32(reg, KEY2) != 0:
            print(f"  !! unlock write failed to 0x{reg:08X}", flush=True)
            return False
    e, ctlr = mem_read(CTLR)
    if e == 0 and (ctlr & 0x8080) == 0:
        print(f"  unlocked (CTLR=0x{ctlr:08X})", flush=True)
        return True
    print(f"  !! unlock failed (CTLR=0x{ctlr:08X} err={e})", flush=True)
    return False


def main():
    global ser
    print("=" * 64, flush=True)
    print("  CH32V203 UNPROTECT (destructive: erases whole chip)", flush=True)
    ser = serial.Serial(PORT, 115200, timeout=0.5)
    time.sleep(0.5)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l, flush=True)

    print("\n-- verify protected state --", flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)
    e, obr = mem_read(0x4002201C)
    e2, ob0 = mem_read(RDPR_ADDR)
    print(f"  FLASH_OBR=0x{obr:08X} RDPRT={1 if obr & 2 else 0}", flush=True)
    print(f"  OB[0]=0x{ob0:08X}", flush=True)
    if not (obr & 2):
        print("  !! not read protected; aborting", flush=True)
        ser.close()
        return 1

    print("\n-- unlock flash --", flush=True)
    if not unlock():
        ser.close()
        return 1

    print("\n-- mass erase --", flush=True)
    if mem_write32(CTLR, 0) != 0:
        print("  !! CTLR=0 write failed", flush=True)
    if mem_write32(CTLR, MER) != 0:
        print("  !! CTLR=MER write failed", flush=True)
    if mem_write32(CTLR, MER | STRT) != 0:
        print("  !! CTLR=MER|STRT write failed", flush=True)
    if not wait_flash("mass erase"):
        ser.close()
        return 1
    print("  mass erase done", flush=True)

    print("\n-- option byte erase --", flush=True)
    if mem_write32(OBKEYR, KEY1) != 0 or mem_write32(OBKEYR, KEY2) != 0:
        print("  !! OBKEYR unlock failed", flush=True)
    if mem_write32(CTLR, OPTER) != 0:
        print("  !! CTLR=OPTER write failed", flush=True)
    if mem_write32(CTLR, OPTER | STRT) != 0:
        print("  !! CTLR=OPTER|STRT write failed", flush=True)
    if not wait_flash("option byte erase"):
        ser.close()
        return 1
    print("  option byte erase done", flush=True)

    print("\n-- program option bytes (all words, RDPR last) --", flush=True)
    # Preserve original values; only change RDPR 0xFF -> 0xA5 (nRDPR 0x00 -> 0x5A).
    # Halves: 0x1FFFF800=0x5AA5(RDPR/nRDPR) 0x1FFFF802=0x3FC0(USER/nUSER)
    #         0x1FFFF804..0x1FFFF80E=0x00FF (WRPR0-3, DATA0/1 = no WPR)
    if mem_write32(CTLR, OPTPG) != 0:
        print("  !! CTLR=OPTPG write failed", flush=True)
    halves = [
        (0x1FFFF802, 0x3FC0),
        (0x1FFFF804, 0x00FF),
        (0x1FFFF806, 0x00FF),
        (0x1FFFF808, 0x00FF),
        (0x1FFFF80A, 0x00FF),
        (0x1FFFF80C, 0x00FF),
        (0x1FFFF80E, 0x00FF),
        (0x1FFFF800, 0x5AA5),
    ]
    for addr, val in halves:
        e = mem_write16(addr, val)
        if e != 0:
            print(f"  !! optbyte write @0x{addr:08X}=0x{val:04X} cmderr={e}", flush=True)
        if not wait_flash(f"optbyte @0x{addr:08X}"):
            ser.close()
            return 1
    time.sleep(0.5)
    if mem_write32(CTLR, 0) != 0:
        print("  !! CTLR clear failed", flush=True)
    print("  option bytes programmed", flush=True)

    e, ob0 = mem_read(RDPR_ADDR)
    e2, ob1 = mem_read(0x1FFFF804)
    print(f"  OB[0]=0x{ob0:08X} OB[1]=0x{ob1:08X} (expect 0xC03F5AA5 / 0xFF00FF00)",
          flush=True)

    print("\n-- reset to reload option bytes --", flush=True)
    dmi_w(DMCONTROL, 0x40000001)
    time.sleep(0.1)
    dmi_w(DMCONTROL, 0x00000001)
    time.sleep(0.3)
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l, flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.005)

    print("\n-- verify unprotect --", flush=True)
    e, obr = mem_read(0x4002201C)
    print(f"  FLASH_OBR=0x{obr:08X} RDPRT={1 if obr & 2 else 0}", flush=True)
    e, f0 = mem_read(0x08000000)
    e2, f1 = mem_read(0x08000400)
    print(f"  FLASH[0x08000000] cmderr={e} = 0x{f0:08X}", flush=True)
    print(f"  FLASH[0x08000400] cmderr={e2} = 0x{f1:08X}", flush=True)
    ok = (obr & 2) == 0 and e == 0 and e2 == 0 and f0 == 0xFFFFFFFF and f1 == 0xFFFFFFFF
    print(("\n  UNPROTECT OK — flash readable and erased" if ok else
           "\n  !! unexpected result"), flush=True)

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())