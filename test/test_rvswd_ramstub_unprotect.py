#!/usr/bin/env python3
"""Load the RAM stub into the CH32V203 and run it to unprotect (option-byte
erase + RDPR regression + restore USER/WRPR/DATA). The stub runs on the core
with normal bus access. Debugger polls DMSTATUS for the stub's final ebreak."""

import sys
import time
import serial
import json

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
STUB_ADDR = 0x20000000
DONE_FLAG, STATUS_FLAG = 0x20001000, 0x20001004
STATR_ERASE, STATR_RDPR = 0x20001008, 0x2000100C
SP_ADDR = 0x20002000

DATA0, DMCONTROL, DMSTATUS, ABSTRACTCS = 0x04, 0x10, 0x11, 0x16
COMMAND = 0x17
HARTINFO = 0x12
ABSTRACTAUTO = 0x18
PROGBUF0, PROGBUF1, PROGBUF2 = 0x20, 0x21, 0x22
PROGBUF3, PROGBUF4, PROGBUF5, PROGBUF6, PROGBUF7 = 0x23, 0x24, 0x25, 0x26, 0x27
CMD_SET_A1, CMD_SET_A0, CMD_SET_A2, CMD_GET_A0, CMD_POSTEXEC = 0x23100B, 0x23100A, 0x23100C, 0x22100A, 0x40000
LW_A0_0_A1, SW_A0_0_A2, ADDI_A2_A2_4 = 0x0005A503, 0x00A62023, 0x00458613
# SW_A0_0_A2: 0x00A62023 = sw a0, 0(a2) (was 0x00A5A623 = sw a0, 12(a1) — a
# decode bug that stored each loaded word back into the DATA0 mmio alias
# instead of SRAM, so the stub never reached 0x20000000 and the verify always
# showed untouched SRAM).
EBREAK = 0x00100073

stub_words = json.loads(open("/tmp/stub_words.py").read().split("=")[1].strip())
ser = None


def send(cmd, delay=0.02, timeout=0.5):
    # Return on ~25ms of line silence, not on the timeout deadline. The probe
    # answers in ms on a healthy link; waiting out the full timeout on every
    # call made each DMI op cost 0.25-0.5s (~2s/word while loading the stub),
    # which looked exactly like a DM wedge when a run hit its time budget.
    ser.reset_input_buffer()
    ser.write((cmd.strip() + "\r\n").encode())
    time.sleep(delay)
    buf = b""
    deadline = time.time() + timeout
    last_data = time.time()
    while time.time() < deadline:
        n = ser.in_waiting
        if n:
            buf += ser.read(n)
            last_data = time.time()
        elif time.time() - last_data > 0.025:
            break
        else:
            time.sleep(0.002)
    return [l.strip() for l in buf.decode(errors="replace").splitlines() if l.strip()]


def dmi_w(reg, val):
    send(f"RVSWD WRITE 0x{reg:02X} 0x{val:08X}")


def dmi_r(reg):
    for l in send(f"RVSWD READ 0x{reg:02X}"):
        if "val=" in l:
            return int(l.split("val=")[1].strip(), 16)
    return None


def set_reg(cmd, val):
    dmi_w(DATA0, val)
    dmi_w(COMMAND, cmd)


def mem_read(addr):
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(PROGBUF0, LW_A0_0_A1)
    dmi_w(PROGBUF1, EBREAK)
    set_reg(CMD_SET_A1, addr)
    dmi_w(COMMAND, CMD_POSTEXEC)
    e = 0
    for l in send(f"RVSWD READ 0x{ABSTRACTCS:02X}"):
        if "val=" in l:
            e = (int(l.split("val=")[1].strip(), 16) >> 8) & 0x7
    dmi_w(ABSTRACTCS, 0x700)
    dmi_w(COMMAND, CMD_GET_A0)
    dmi_w(ABSTRACTCS, 0x700)
    return e, dmi_r(DATA0)


def write_core_reg(regno, val):
    dmi_w(DATA0, val)
    dmi_w(COMMAND, 0x10000 | 0x20000 | 0x00200000 | regno)


def read_core_reg(regno):
    dmi_w(COMMAND, 0x20000 | 0x00200000 | regno)
    time.sleep(0.05)
    return dmi_r(DATA0)


def main():
    global ser
    print("=" * 64, flush=True)
    print("  CH32V203 RAM-STUB UNPROTECT", flush=True)
    ser = serial.Serial(PORT, 115200, timeout=0.5)
    time.sleep(0.5)
    # Drain any backlog left by interrupted runs: the probe processes serial
    # commands one at a time and keeps going after a host-side kill, so stale
    # RVSWD responses may still be streaming. Read until ~1s of silence.
    ser.reset_input_buffer()
    silent_since = time.time()
    while time.time() - silent_since < 1.0:
        n = ser.in_waiting
        if n:
            ser.read(n)
            silent_since = time.time()
        else:
            time.sleep(0.05)
    ser.reset_input_buffer()
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.8, timeout=3.0):
        print("  " + l, flush=True)

    print("\n-- halt core --", flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(50):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.01)
    print(f"  DMSTATUS=0x{dmi_r(DMSTATUS):08X}", flush=True)

    print(f"\n-- load stub ({len(stub_words)} words) to SRAM 0x{STUB_ADDR:08X} via per-word POSTEXEC --", flush=True)
    # The WCH DM wedges after ~30-80 RAPID AUTOEXECDATA executions (F9), but
    # individual POSTEXEC writes with register setup between them (the pattern
    # the flash-erase path uses) do not wedge. Load one word per POSTEXEC:
    #   progbuf = sw a0, 0(a1); ebreak   (a0 = value, a1 = address)
    SW_A0_0_A1 = 0x00A5A023  # sw a0, 0(a1)

    def reset_dm():
        # haltreq pulse (same as rvswd_transport_reset: 0x80000001 x2, then
        # dmactive-only) — snaps a wedged DM out (F9 recovery), then re-assert
        # haltreq and wait for ALLHALTED so the core stays stopped.
        dmi_w(DMCONTROL, 0x80000001)
        dmi_w(DMCONTROL, 0x80000001)
        dmi_w(DMCONTROL, 0x00000001)
        dmi_w(DMCONTROL, 0x80000001)
        for _ in range(50):
            if (dmi_r(DMSTATUS) or 0) & (1 << 9):
                break
            time.sleep(0.01)

    loaded = 0
    while loaded < len(stub_words):
        dmi_w(ABSTRACTCS, 0x700)
        dmi_w(PROGBUF0, SW_A0_0_A1)
        dmi_w(PROGBUF1, EBREAK)
        end = min(loaded + 32, len(stub_words))
        for i in range(loaded, end):
            set_reg(CMD_SET_A1, STUB_ADDR + i * 4)
            set_reg(CMD_SET_A0, stub_words[i])
            dmi_w(COMMAND, CMD_POSTEXEC)
            # Wait for the abstract command to finish before the next word —
            # firing commands into a busy DM is what wedges it (the flash-erase
            # path always waits BUSY clear, and it never wedges).
            for _ in range(50):
                v = dmi_r(ABSTRACTCS)
                if v is None:
                    break
                if not (v & 0x1000):
                    break
                time.sleep(0.002)
            if (i + 1) % 16 == 0:
                print(f"  loaded {i+1}/{len(stub_words)}", flush=True)
        loaded = end
        if loaded < len(stub_words):
            # Wedge guard: reset the DM between batches to stay clear of the
            # ~30-80 consecutive-execution wedge, then reload the program.
            reset_dm()

    print("  verifying stub words in SRAM ...", flush=True)
    ok = True
    for i in range(0, 12):
        _, v = mem_read(STUB_ADDR + i * 4)
        good = v == stub_words[i]
        if not good:
            ok = False
        print(f"    [{(STUB_ADDR+i*4):#010x}] 0x{v:08X} expected 0x{stub_words[i]:08X} {'OK' if good else 'MISMATCH'}", flush=True)
    print("  stub load " + ("verified" if ok else "MISMATCH!"), flush=True)

    print("\n-- set SP and PC --", flush=True)
    write_core_reg(0x1002, SP_ADDR)
    write_core_reg(0x7B1, STUB_ADDR)
    pc = read_core_reg(0x7B1)
    print(f"  PC=0x{pc:08X}", flush=True)

    print("  sending resume ...", flush=True)
    send(f"RVSWD WRITE 0x10 0x40000001", timeout=1.0)
    print("  resume write returned", flush=True)
    time.sleep(10)
    print("  slept 10s", flush=True)
    send(f"CH32 DETECT mcu=ch32v203", delay=1.0, timeout=5.0)
    print("  DETECT returned", flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(50):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.01)

    done_e, done = mem_read(DONE_FLAG)
    st_e, status = mem_read(STATUS_FLAG)
    print(f"  done_flag  cmderr={done_e} = 0x{done:08X} (2=success 0xFF=fail)", flush=True)
    print(f"  status     cmderr={st_e}  = 0x{status:08X} (1=started 2=success 0xFF=fail)", flush=True)
    _, se = mem_read(STATR_ERASE)
    _, sr = mem_read(STATR_RDPR)
    # STATR: bit2=PGERR bit4=WRPRTERR bit5=EOP
    def statr(v):
        f = []
        if v & 0x04: f.append("PGERR")
        if v & 0x10: f.append("WRPRTERR")
        if v & 0x20: f.append("EOP")
        return "+".join(f) if f else "-"
    print(f"  statr after option erase = 0x{se:08X} [{statr(se)}]", flush=True)
    print(f"  statr after RDPR program = 0x{sr:08X} [{statr(sr)}]", flush=True)

    print("\n-- reset target to reload option bytes --", flush=True)
    # ndmreset (dmactive|ndmreset): a haltreq pulse does NOT reload the
    # option bytes on the V203 — OBR/WPR keep the stale loaded values until
    # a real reset (verified empirically).
    dmi_w(DMCONTROL, 0x00000003)
    time.sleep(0.3)
    dmi_w(DMCONTROL, 0x00000001)
    time.sleep(0.5)
    for l in send("CH32 DETECT mcu=ch32v203", delay=0.8, timeout=3.0):
        print("  " + l, flush=True)
    dmi_w(DMCONTROL, 0x80000001)
    for _ in range(50):
        if (dmi_r(DMSTATUS) or 0) & (1 << 9):
            break
        time.sleep(0.01)

    print("\n-- verify unprotect --", flush=True)
    # V20x option-byte layout: 0x00=RDPR+USER 0x04=Data0+Data1 0x08=WRPR0+1 0x0C=WRPR2+3
    for label, addr in [("OBR", 0x4002201C), ("WPR", 0x40022020),
                        ("OB[0x00]", 0x1FFFF800), ("OB[0x04]", 0x1FFFF804),
                        ("OB[0x08]", 0x1FFFF808), ("OB[0x0C]", 0x1FFFF80C),
                        ("FLASH@0x08000000", 0x08000000), ("FLASH@0x08000400", 0x08000400)]:
        e, v = mem_read(addr)
        print(f"  {label:18s} cmderr={e} value=0x{v:08X}", flush=True)
    e, v = mem_read(0x4002201C)
    _, wpr = mem_read(0x40022020)
    ok = done == 2 and (v & 2) == 0
    if ok:
        print("  UNPROTECT OK — RDP cleared", flush=True)
        print(f"  WPR=0x{wpr:08X} (0xFFFFFFFF = all blocks unprotected)", flush=True)
    else:
        print("  !! RDP still active or stub failed", flush=True)

    dmi_w(DMCONTROL, 0x40000001)
    dmi_w(DMCONTROL, 0x00000001)
    ser.close()
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())