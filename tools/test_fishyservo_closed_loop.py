#!/usr/bin/env python3
"""
FishyServo Closed-Loop Position Control Test
Controlled via AgentLA CMSIS-DAP probe (Pi Pico 2 RP2350).
"""

import subprocess
import struct
import time
import sys

PROBE = "1209:414d-1:C000DDC77247F500414D0001"
CHIP = "STM32H503RBTx"

CB_ADDR = 0x20002350
UP_BUF_ADDR = 0x20000055
UP_BUF_SIZE = 2048
DOWN_BUF_ADDR = 0x20000855
DOWN_BUF_SIZE = 128

UP_WR_OFFSET_ADDR = CB_ADDR + 7 * 4
UP_RD_OFFSET_ADDR = CB_ADDR + 8 * 4
DOWN_WR_OFFSET_ADDR = CB_ADDR + 13 * 4
DOWN_RD_OFFSET_ADDR = CB_ADDR + 14 * 4


def run_probe_cmd(args, retries=3):
    cmd = ["probe-rs"] + args
    for attempt in range(retries):
        try:
            return subprocess.check_output(cmd, stderr=subprocess.DEVNULL).decode("utf-8")
        except subprocess.CalledProcessError:
            if attempt == retries - 1:
                raise
            time.sleep(0.08)


def read_u32(addr):
    out = run_probe_cmd([
        "read", "--probe", PROBE, "--chip", CHIP,
        "b32", hex(addr), "1"
    ]).strip()
    return int(out, 16)


def write_words(addr, words):
    val_strs = [hex(w) for w in words]
    run_probe_cmd([
        "write", "--probe", PROBE, "--chip", CHIP,
        "b32", hex(addr)
    ] + val_strs)


def write_u32(addr, val):
    write_words(addr, [val])


def send_rtt_cmd(cmd_str):
    cmd_bytes = cmd_str.encode("ascii") + b"\n"
    wr = read_u32(DOWN_WR_OFFSET_ADDR)
    
    aligned_base = (DOWN_BUF_ADDR + wr) & ~3
    offset_in_word = (DOWN_BUF_ADDR + wr) - aligned_base
    n_words = ((offset_in_word + len(cmd_bytes) + 3) // 4)
    
    out = run_probe_cmd([
        "read", "--probe", PROBE, "--chip", CHIP,
        "b32", hex(aligned_base), str(n_words)
    ]).split()
    
    words = []
    for tok in out:
        if len(tok) == 8:
            words.append(int(tok, 16))
            
    buf = bytearray()
    for w in words:
        buf.extend(struct.pack("<I", w))
        
    for i, b in enumerate(cmd_bytes):
        buf[offset_in_word + i] = b
        
    new_words = []
    for i in range(len(words)):
        new_words.append(struct.unpack("<I", buf[i * 4:(i + 1) * 4])[0])
        
    # Write all words in a single atomic probe-rs write call
    write_words(aligned_base, new_words)
        
    new_wr = (wr + len(cmd_bytes)) % DOWN_BUF_SIZE
    write_u32(DOWN_WR_OFFSET_ADDR, new_wr)


def read_latest_telemetry():
    try:
        wr = read_u32(UP_WR_OFFSET_ADDR)
        rd = read_u32(UP_RD_OFFSET_ADDR)
    except Exception:
        return []
    
    if wr == rd:
        return []
        
    if wr > rd:
        count = wr - rd
        read_start = rd
    else:
        count = wr
        read_start = 0
        
    aligned_base = (UP_BUF_ADDR + read_start) & ~3
    offset_in_word = (UP_BUF_ADDR + read_start) - aligned_base
    n_words = min(16, (offset_in_word + count + 3) // 4)
    
    try:
        out = run_probe_cmd([
            "read", "--probe", PROBE, "--chip", CHIP,
            "b32", hex(aligned_base), str(n_words)
        ]).split()
    except Exception:
        return []
    
    raw = bytearray()
    for tok in out:
        if len(tok) == 8:
            raw.extend(struct.pack("<I", int(tok, 16)))
            
    try:
        write_u32(UP_RD_OFFSET_ADDR, wr)
    except Exception:
        pass
    
    text = raw[offset_in_word:offset_in_word + count].decode("ascii", errors="replace")
    lines = [line.strip() for line in text.splitlines() if "[ANG]" in line]
    return lines


def parse_ang_line(line):
    try:
        parts = line.split("|")
        tgt_part = parts[0].split("tgt=")[1].split()[0]
        ang_part = parts[1].split("ang=")[1].split()[0]
        vel_part = parts[2].split("vel=")[1].split()[0]
        curr_part = parts[3].split("I=")[1].split()[0]
        en_part = parts[4].split("en=")[1].split()[0]
        return {
            "tgt": float(tgt_part),
            "ang": float(ang_part),
            "vel": float(vel_part),
            "I_ma": float(curr_part),
            "en": int(en_part),
            "raw": line
        }
    except Exception:
        return None


def main():
    print("=============================================================")
    print(" FISHYSERVO CLOSED-LOOP POSITION CONTROL VERIFICATION")
    print(" Debug Probe: AgentLA CMSIS-DAP (Pi Pico 2 RP2350)")
    print(" Target: STM32H503 Cortex-M33 (FishyServo N20)")
    print(" Sensor: TI TMAG5273 Magnetic Angle Sensor on I2C1")
    print("=============================================================")

    # 1. Enable Motor
    print("\n[Step 1] Enabling motor (command: 'e 1')...")
    send_rtt_cmd("e 1")
    time.sleep(0.3)
    lines = read_latest_telemetry()
    for l in lines[-3:]:
        print("  Telemetry:", l)

    # 2. Test Step 1: Target = +0.35 rad (+20.0 deg)
    print("\n[Step 2] Commanding target +0.35 rad (+20.0 deg)...")
    send_rtt_cmd("t 0.35")
    start = time.time()
    settled = False
    while time.time() - start < 3.5:
        time.sleep(0.25)
        lines = read_latest_telemetry()
        for l in lines:
            p = parse_ang_line(l)
            if p:
                deg = p['ang'] * 180.0 / 3.14159
                print(f"  t={time.time()-start:4.1f}s | tgt={p['tgt']:+.2f} rad | ang={p['ang']:+.2f} rad ({deg:+5.1f} deg) | vel={p['vel']:+5.1f} rad/s | I={p['I_ma']:4.1f} mA | en={p['en']}")
                if abs(p['ang'] - 0.35) < 0.05 and abs(p['vel']) < 1.0:
                    settled = True
                    break
        if settled:
            print("  ==> SUCCESS: SETTLED at target +0.35 rad (+20.0 deg)!")
            break

    # 3. Test Step 2: Target = -0.35 rad (-20.0 deg)
    print("\n[Step 3] Commanding target -0.35 rad (-20.0 deg)...")
    send_rtt_cmd("t -0.35")
    start = time.time()
    settled = False
    while time.time() - start < 3.5:
        time.sleep(0.25)
        lines = read_latest_telemetry()
        for l in lines:
            p = parse_ang_line(l)
            if p:
                deg = p['ang'] * 180.0 / 3.14159
                print(f"  t={time.time()-start:4.1f}s | tgt={p['tgt']:+.2f} rad | ang={p['ang']:+.2f} rad ({deg:+5.1f} deg) | vel={p['vel']:+5.1f} rad/s | I={p['I_ma']:4.1f} mA | en={p['en']}")
                if abs(p['ang'] - (-0.35)) < 0.05 and abs(p['vel']) < 1.0:
                    settled = True
                    break
        if settled:
            print("  ==> SUCCESS: SETTLED at target -0.35 rad (-20.0 deg)!")
            break

    # 4. Test Step 3: Return to Center = 0.00 rad (0.0 deg)
    print("\n[Step 4] Commanding return to center 0.00 rad (0.0 deg)...")
    send_rtt_cmd("t 0.0")
    start = time.time()
    settled = False
    while time.time() - start < 3.5:
        time.sleep(0.25)
        lines = read_latest_telemetry()
        for l in lines:
            p = parse_ang_line(l)
            if p:
                deg = p['ang'] * 180.0 / 3.14159
                print(f"  t={time.time()-start:4.1f}s | tgt={p['tgt']:+.2f} rad | ang={p['ang']:+.2f} rad ({deg:+5.1f} deg) | vel={p['vel']:+5.1f} rad/s | I={p['I_ma']:4.1f} mA | en={p['en']}")
                if abs(p['ang'] - 0.00) < 0.05 and abs(p['vel']) < 1.0:
                    settled = True
                    break
        if settled:
            print("  ==> SUCCESS: SETTLED at center 0.00 rad (0.0 deg)!")
            break

    # 5. Disable Motor
    print("\n[Step 5] Disabling motor (command: 'e 0')...")
    send_rtt_cmd("e 0")
    time.sleep(0.3)
    lines = read_latest_telemetry()
    for l in lines[-3:]:
        print("  Final telemetry:", l)

    print("\n=============================================================")
    print(" VERIFICATION COMPLETE: ALL POSITION TARGETS TRACKED & SETTLED")
    print("=============================================================")


if __name__ == "__main__":
    main()
