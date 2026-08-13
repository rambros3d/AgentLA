#!/usr/bin/env python3
"""
Test two hypotheses for the CH32V203 flash-load exception (cmderr=3):

A) minichlink-style read program (address via DATA1 magic alias, c.lw/c.sw)
   vs our simple lw a0,0(a1) program - does the mechanism matter?
B) reset-then-halt (PicoRVD reset()) before reading - does halt-at-reset help?

Each phase reads SRAM control + flash on the SAME probe.
"""

import sys
import time
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

DATA0, DATA1, DMCONTROL, DMSTATUS, HARTINFO = 0x04, 0x05, 0x10, 0x11, 0x12
ABSTRACTCS, COMMAND, ABSTRACTAUTO = 0x16, 0x17, 0x18
PROGBUF0, PROGBUF1, PROGBUF2 = 0x20, 0x21, 0x22

CMD_SET_A0, CMD_SET_A1, CMD_POSTEXEC = 0x23100A, 0x23100B, 0x40000
CMD_GET_A0 = 0x22100A
CMD_POSTEXEC_32 = 0x00240000            # minichlink: postexec + aarsize32

LW_A0_0_A1, EBREAK = 0x0005A503, 0x00100073
M_LW_X8_0_X11 = 0x40044180              # c.lw x8, 0(x11)
M_ADDI_SW_X10 = 0xc1040411              # c.addi x8,4; c.sw x9,0(x10)
M_SW_X11_EBRK = 0x9002c180              # c.sw x8,0(x11); c.ebreak

ser = None


def send(cmd, delay=0.04, timeout=1.0):
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


def acs():
    a = dmi_r(ABSTRACTCS)
    return ((a >> 8) & 0x7, a >> 12 & 1, (a >> 1) & 1) if a is not None else (-1, -1, -1)


def set_reg(cmd, val):
    dmi_w(DATA0, val)
    dmi_w(COMMAND, cmd)


def do_read(load_prog_fn, addr):
    """Run one read: returns (cmderr, data0)."""
    load_prog_fn()
    set_reg(CMD_SET_A1, addr)
    dmi_w(COMMAND, CMD_POSTEXEC)
    err, busy, bit1 = acs()
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(COMMAND, CMD_GET_A0)
    dmi_w(ABSTRACTCS, 0x700)
    val = dmi_r(DATA0)
    return err, val


def probe_simple():
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, LW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)


def probe_minichlink():
    hart = dmi_r(HARTINFO)
    d0 = 0xE0000000 | (hart & 0x7FF)
    set_reg(CMD_SET_A0, d0)        # x10 = DATA0 magic
    set_reg(CMD_SET_A1, d0 + 4)    # x11 = DATA1 magic
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, M_LW_X8_0_X11)
    dmi_w(PROGBUF1, M_ADDI_SW_X10)
    dmi_w(PROGBUF2, M_SW_X11_EBRK)


def do_read_minichlink(addr):
    """minichlink DefaultReadWord single-shot: DATA1=addr; COMMAND=0x00240000"""
    probe_minichlink()
    dmi_w(DATA1, addr)
    dmi_w(COMMAND, CMD_POSTEXEC_32)
    err, busy, bit1 = acs()
    dmi_w(ABSTRACTCS, 0x700)
    val = dmi_r(DATA0)
    return err, val


def halt():
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            return
        time.sleep(0.005)
    print("  !! halt timeout", flush=True)


def reset_halt():
    """PicoRVD reset(): halt, ndmreset, clear, ackhavereset, halt again."""
    dmi_w(DMCONTROL, 0x80000001)
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x80000003)   # ndmreset | haltreq | dmactive
    time.sleep(0.1)
    dmi_w(DMCONTROL, 0x80000001)
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x90000001)   # ackhavereset | haltreq | dmactive
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x00000001)
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x80000001)   # re-halt
    for _ in range(20):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            return
        time.sleep(0.005)
    print("  !! reset-halt timeout", flush=True)


def resume():
    dmi_w(DMCONTROL, 0x40000001)
    time.sleep(0.05)
    dmi_w(DMCONTROL, 0x00000001)


def main():
    global ser
    print("=" * 68, flush=True)
    print("  CH32V203 flash-load exception: mechanism & halt-state tests", flush=True)
    ser = serial.Serial(PORT, BAUD, timeout=0.5)
    time.sleep(0.5)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.5):
        print("  " + l, flush=True)

    halt()
    print("\n-- A1: simple lw program, plain halt --", flush=True)
    print(f"    SRAM  0x20000000: cmderr={do_read(probe_simple, 0x20000000)[0]}", flush=True)
    e, v = do_read(probe_simple, 0x08000000)
    print(f"    FLASH 0x08000000: cmderr={e} data0=0x{v:08X}", flush=True)

    print("\n-- A2: minichlink DATA1-magic program, plain halt --", flush=True)
    e, v = do_read_minichlink(0x20000000)
    print(f"    SRAM  0x20000000: cmderr={e} data0=0x{v:08X}", flush=True)
    e, v = do_read_minichlink(0x08000000)
    print(f"    FLASH 0x08000000: cmderr={e} data0=0x{v:08X}", flush=True)

    resume()
    print("\n-- B: reset+halt, then simple program --", flush=True)
    reset_halt()
    e, v = do_read(probe_simple, 0x20000000)
    print(f"    SRAM  0x20000000: cmderr={e} data0=0x{v:08X}", flush=True)
    e, v = do_read(probe_simple, 0x08000000)
    print(f"    FLASH 0x08000000: cmderr={e} data0=0x{v:08X}", flush=True)

    resume()
    ser.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())