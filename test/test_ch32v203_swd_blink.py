#!/usr/bin/env python3
"""
CH32V203 SWD Communication Test + Blink Firmware Flash & BMP Function Verification

Tests:
  1. SWD communication via AgentLA RV-SWD driver (2-wire)
  2. ESIG chip ID probe (BMP-compatible: 0x1FFFF704)
  3. Flash sector erase + program + readback verify
  4. Flash a real RISC-V blink firmware (RCC + GPIOA + delay loop)
  5. Flash verify via FLASH VERIFY command
  6. Multi-address flash read

BMP Reference (ch32vx.c):
  - ESIG chip_id at 0x1FFFF704 (family mask 0xFFF at bits [31:20])
  - Flash capacity at 0x1FFFF7E0 (16-bit, KiB units)
  - UID at 0x1FFFF7E8, 0x1FFFF7EC, 0x1FFFF7F0 (96-bit)
"""

import os
import sys
import time
import subprocess
import struct
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200
PASS_COUNT = 0
FAIL_COUNT = 0


# ─── RISC-V Instruction Encoding Helpers ─────────────────────────────────────

def I_TYPE(opcode, rd, funct3, rs1, imm12):
    """I-type: imm[11:0] | rs1 | funct3 | rd | opcode"""
    imm12 = imm12 & 0xFFF
    return (imm12 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode


def S_TYPE(opcode, funct3, rs1, rs2, imm12):
    """S-type: imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode"""
    imm12 = imm12 & 0xFFF
    return (((imm12 >> 5) & 0x7F) << 25) | (rs2 << 20) | (rs1 << 15) | \
           (funct3 << 12) | ((imm12 & 0x1F) << 7) | opcode


def B_TYPE(opcode, funct3, rs1, rs2, imm13):
    """B-type: imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode"""
    imm13 = imm13 & 0x1FFE  # bit 0 always 0
    return (((imm13 >> 12) & 1) << 31) | (((imm13 >> 5) & 0x3F) << 25) | \
           (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | \
           (((imm13 >> 11) & 1) << 7) | (((imm13 >> 1) & 0xF) << 8) | opcode


def U_TYPE(opcode, rd, imm20):
    """U-type: imm[31:12] | rd | opcode"""
    return ((imm20 & 0xFFFFF) << 12) | (rd << 7) | opcode


# Register aliases
X0, X1, X2 = 0, 1, 2
X5, X6, X7 = 5, 6, 7
X10, X11, X12 = 10, 11, 12

# Opcodes
OPC_REG   = 0x33
OPC_IMM   = 0x13
OPC_LOAD  = 0x03
OPC_STORE = 0x23
OPC_BRANCH = 0x63
OPC_LUI   = 0x37

# Funct3
F3_ADD = 0; F3_SLL = 1; F3_SLTU = 3; F3_XOR = 4; F3_SRL = 5; F3_OR = 6; F3_AND = 7
F3_BEQ = 0; F3_BNE = 1; F3_BLTU = 6; F3_BGEU = 7; F3_BGE = 5
F3_LW = 2; F3_SW = 2; F3_LBU = 4


# ─── CH32V203 Blink Firmware ─────────────────────────────────────────────────
#
# Memory map:
#   RCC_APB2ENR  @ 0x40021018  (bit 2 = IOPAEN for GPIOA clock)
#   GPIOA_CRL    @ 0x40010800  (pin 0: CNF=00, MODE=11 -> push-pull 50MHz)
#   GPIOA_ODR    @ 0x4001080C  (output data register, bit 0 = PA0)
#
# Program flow:
#   [0x00-0x08] Vector table: Reset->init, NMI->0, HardFault->0
#   [0x0C] Init: enable GPIOA clock, configure PA0 as output
#   [0x34] Loop: toggle PA0 via ODR XOR, delay, repeat

def build_blink_bin():
    code = []

    # ── Vector table ──
    code.append(0x08000004)   # [0x00] Reset vector -> init code
    code.append(0x00000000)   # [0x04] NMI vector
    code.append(0x00000000)   # [0x08] HardFault vector

    # ── Init @ 0x0800000C ──
    # 1) Enable GPIOA clock: RCC_APB2ENR |= (1 << 2)
    code.append(U_TYPE(OPC_LUI, X10, 0x40021))              # [3] a0 = 0x40021000
    code.append(I_TYPE(OPC_LOAD, X11, F3_LW, X10, 0x018))   # [4] a1 = RCC_APB2ENR
    code.append(I_TYPE(OPC_IMM, X11, F3_OR, X11, 0x004))    # [5] a1 |= IOPAEN
    code.append(S_TYPE(OPC_STORE, F3_SW, X10, X11, 0x018))  # [6] RCC_APB2ENR = a1

    # 2) Configure PA0: GPIOA_CRL = (CRL & ~0xF) | 0x3
    code.append(U_TYPE(OPC_LUI, X10, 0x40010))              # [7] a0 = 0x40010000
    code.append(I_TYPE(OPC_IMM, X10, F3_ADD, X10, 0x800))   # [8] a0 = 0x40010800 (GPIOA)
    code.append(I_TYPE(OPC_LOAD, X11, F3_LW, X10, 0x000))   # [9] a1 = GPIOA_CRL
    code.append(I_TYPE(OPC_IMM, X11, F3_AND, X11, 0xFFF0))  # [10] a1 &= ~0xF (clear pin 0)
    code.append(I_TYPE(OPC_IMM, X11, F3_OR, X11, 0x003))    # [11] a1 |= 0x3 (50MHz push-pull)
    code.append(S_TYPE(OPC_STORE, F3_SW, X10, X11, 0x000))  # [12] GPIOA_CRL = a1

    # ── Main loop @ 0x08000034 (word 13) ──
    # Toggle PA0: read ODR, XOR with 1, write back
    code.append(I_TYPE(OPC_LOAD, X11, F3_LW, X10, 0x00C))   # [13] a1 = GPIOA_ODR
    code.append(I_TYPE(OPC_IMM, X11, F3_XOR, X11, 0x001))   # [14] a1 ^= 1 (toggle PA0)
    code.append(S_TYPE(OPC_STORE, F3_SW, X10, X11, 0x00C))  # [15] GPIOA_ODR = a1

    # Delay loop (~0x4C000 iterations)
    code.append(U_TYPE(OPC_LUI, X12, 0x0004C))              # [16] a2 = 0x0004C000
    # delay:
    code.append(I_TYPE(OPC_IMM, X12, F3_ADD, X12, -1 & 0xFFF))  # [17] a2 -= 1
    code.append(B_TYPE(OPC_BRANCH, F3_BNE, X12, X0, -4))    # [18] bnez a2, [17]

    # Jump back to main loop (word 13)
    # From word 19 to word 13: offset = (13-19)*4 = -24 bytes
    code.append(B_TYPE(OPC_BRANCH, F3_BGE, X0, X0, -24))    # [19] j main_loop (always)

    # Pad to 32 words (128 bytes) with NOPs
    NOP = I_TYPE(OPC_IMM, X0, F3_ADD, X0, 0)
    while len(code) < 32:
        code.append(NOP)

    # Convert to little-endian bytes
    binary = bytearray()
    for word in code[:32]:
        binary += struct.pack('<I', word & 0xFFFFFFFF)
    return binary


blink_bin = build_blink_bin()
assert len(blink_bin) == 128, f"Blink binary is {len(blink_bin)} bytes, expected 128"
hex_data = blink_bin.hex()


# ─── Serial Helpers ──────────────────────────────────────────────────────────

def cleanup_port_locks(port):
    try:
        subprocess.run(["fuser", "-k", port], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        time.sleep(0.3)
    except Exception:
        pass


def send_cmd(ser, cmd, timeout=3.0):
    """Send serial command, return list of response lines."""
    ser.reset_input_buffer()
    ser.write((cmd.strip() + '\r\n').encode())
    ser.flush()
    time.sleep(0.2)
    deadline = time.time() + timeout
    lines = []
    while time.time() < deadline:
        if ser.in_waiting:
            line = ser.readline().decode(errors='replace').strip()
            if line:
                lines.append(line)
                if line.startswith("OK") or line.startswith("ERR"):
                    break
        else:
            time.sleep(0.05)
    return lines


def check(label, condition, detail=""):
    global PASS_COUNT, FAIL_COUNT
    if condition:
        PASS_COUNT += 1
        print(f"  [PASS] {label}")
    else:
        FAIL_COUNT += 1
        print(f"  [FAIL] {label} {detail}")


# ─── Main Test ───────────────────────────────────────────────────────────────

def main():
    global PASS_COUNT, FAIL_COUNT

    print("=" * 65)
    print(" CH32V203 SWD + Blink Flash + BMP Function Verification")
    print("=" * 65)
    print(f" Port: {PORT}")
    print()

    cleanup_port_locks(PORT)

    try:
        ser = serial.Serial(PORT, BAUD, timeout=2, rtscts=False, dsrdtr=False)
        ser.dtr = True
        ser.rts = True
    except Exception as e:
        print(f"  [FATAL] Cannot open {PORT}: {e}")
        return False

    time.sleep(0.5)

    # ── Stage 1: IDENTIFY ────────────────────────────────────────────────────
    print("Stage 1: Device Handshake (IDENTIFY)")
    lines = send_cmd(ser, "IDENTIFY")
    id_banner = any("AGENTLA" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("AgentLA firmware responds", id_banner)

    # ── Stage 2: SWD Pin Check ───────────────────────────────────────────────
    print("\nStage 2: SWD Pin Configuration")
    lines = send_cmd(ser, "SWD PINS")
    pins_ok = any("SWCLK=4" in l and "SWDIO=5" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("SWD pins correct (SWCLK=4, SWDIO=5, NRESET=6)", pins_ok)

    # ── Stage 3: CH32V203 Target Probe ───────────────────────────────────────
    print("\nStage 3: CH32V203 Target Probe (2-Wire RV-SWD)")
    lines = send_cmd(ser, "CH32 DETECT mode=2wire", timeout=5.0)
    probe_ok = False
    chip_id = 0
    flash_size = 0
    page_size = 0
    for l in lines:
        print(f"  > {l}")
        if l.startswith("OK CH32 DETECT"):
            probe_ok = True
            for part in l.split():
                if part.startswith("chip_id="):
                    chip_id = int(part.split("=")[1], 0)
                elif part.startswith("flash_size="):
                    flash_size = int(part.split("=")[1])
                elif part.startswith("page_size="):
                    page_size = int(part.split("=")[1])

    check("CH32V203 target detected via RV-SWD", probe_ok)
    check(f"Chip ID = 0x{chip_id:08X}", chip_id != 0 and chip_id != 0xFFFFFFFF)

    # BMP compatibility: family field at bits [31:20]
    family = (chip_id >> 20) & 0xFFF
    check(f"BMP family ID = 0x{family:03X} (expected 0x203 for CH32V203)", family == 0x203)

    check(f"Flash size = {flash_size} bytes", flash_size > 0)
    check(f"Page size = {page_size} bytes", page_size == 256)

    # ── Stage 4: Flash Erase ─────────────────────────────────────────────────
    print("\nStage 4: Flash Sector Erase (128B @ 0x08000000)")
    lines = send_cmd(ser, "CH32 ERASE 0x08000000 128", timeout=5.0)
    erase_ok = any("OK CH32 ERASED" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("Flash erase successful", erase_ok)

    # ── Stage 5: Flash Blink Firmware ────────────────────────────────────────
    print(f"\nStage 5: Flash Blink Firmware ({len(blink_bin)} bytes @ 0x08000000)")
    lines = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_data}", timeout=5.0)
    flash_ok = any("OK CH32 FLASH WROTE" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("Flash write successful", flash_ok)

    # ── Stage 6: Readback & Byte Parity ──────────────────────────────────────
    print("\nStage 6: Flash Readback & Byte-Level Verification")
    lines = send_cmd(ser, "FLASH READ 0x08000000 128", timeout=3.0)
    readback_bytes = None
    for l in lines:
        if l.startswith("FLASH DATA "):
            hex_read = l.replace("FLASH DATA ", "").replace(",", "").strip()
            try:
                readback_bytes = bytes.fromhex(hex_read)
            except Exception as ex:
                check("Readback hex parse", False, str(ex))

    if readback_bytes is not None:
        if readback_bytes == blink_bin:
            check(f"Readback matches ({len(readback_bytes)} bytes identical)", True)
        else:
            for i in range(min(len(readback_bytes), len(blink_bin))):
                if readback_bytes[i] != blink_bin[i]:
                    check("Readback match", False,
                          f"mismatch at 0x{i:02X}: expected 0x{blink_bin[i]:02X} got 0x{readback_bytes[i]:02X}")
                    break
    else:
        check("Readback data received", False, "no FLASH DATA line")

    # ── Stage 7: FLASH VERIFY ────────────────────────────────────────────────
    print("\nStage 7: FLASH VERIFY (firmware-side comparison)")
    lines = send_cmd(ser, f"FLASH VERIFY 0x08000000 {hex_data}", timeout=3.0)
    verify_ok = any("OK FLASH VERIFY PASS" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("FLASH VERIFY PASS", verify_ok)

    # ── Stage 8: Flash Content Analysis ──────────────────────────────────────
    print("\nStage 8: Flash Content Analysis")
    if readback_bytes:
        reset_vec = struct.unpack('<I', readback_bytes[0:4])[0]
        check(f"Reset vector = 0x{reset_vec:08X} (expected 0x08000004)", reset_vec == 0x08000004)

        nmi_vec = struct.unpack('<I', readback_bytes[4:8])[0]
        check(f"NMI vector = 0x00000000", nmi_vec == 0x00000000)

        hf_vec = struct.unpack('<I', readback_bytes[8:12])[0]
        check(f"HardFault vector = 0x00000000", hf_vec == 0x00000000)

        # Check init code: LUI a0, 0x40021
        init_word = struct.unpack('<I', readback_bytes[12:16])[0]
        expected_lui = U_TYPE(OPC_LUI, X10, 0x40021) & 0xFFFFFFFF
        check(f"Init: LUI a0, 0x40021 @ 0x0800000C", init_word == expected_lui,
              f"got 0x{init_word:08X}")

        # Check main loop: LW a1, GPIOA_ODR
        loop_word = struct.unpack('<I', readback_bytes[52:56])[0]
        expected_lw = I_TYPE(OPC_LOAD, X11, F3_LW, X10, 0x00C) & 0xFFFFFFFF
        check(f"Loop: LW a1, GPIOA_ODR @ 0x08000034", loop_word == expected_lw,
              f"got 0x{loop_word:08X}")
    else:
        check("Content analysis skipped", False, "no readback data")

    # ── Stage 9: Re-probe After Flash ────────────────────────────────────────
    print("\nStage 9: Target Re-probe After Flash")
    lines = send_cmd(ser, "CH32 DETECT mode=2wire", timeout=5.0)
    reprobe_ok = any("OK CH32 DETECT" in l for l in lines)
    for l in lines:
        print(f"  > {l}")
    check("Target still accessible after flash", reprobe_ok)

    # ── Stage 10: Multi-Address Flash Read ───────────────────────────────────
    print("\nStage 10: Multi-Address Flash Read")
    for addr, expected_len in [(0x08000000, 32), (0x08000040, 32), (0x08000060, 32)]:
        lines = send_cmd(ser, f"FLASH READ 0x{addr:08X} {expected_len}", timeout=3.0)
        data_ok = False
        for l in lines:
            if l.startswith("FLASH DATA "):
                hex_read = l.replace("FLASH DATA ", "").replace(",", "").strip()
                try:
                    data = bytes.fromhex(hex_read)
                    data_ok = len(data) == expected_len
                except Exception:
                    pass
        check(f"FLASH READ 0x{addr:08X} {expected_len}B", data_ok)

    # ── Summary ──────────────────────────────────────────────────────────────
    ser.close()

    print()
    print("=" * 65)
    total = PASS_COUNT + FAIL_COUNT
    print(f" RESULTS: {PASS_COUNT}/{total} passed, {FAIL_COUNT}/{total} failed")
    if FAIL_COUNT == 0:
        print(" ALL TESTS PASSED")
    else:
        print(f" {FAIL_COUNT} TEST(S) FAILED")
    print("=" * 65)

    if FAIL_COUNT == 0:
        print()
        print("Blink firmware loaded at 0x08000000.")
        print("Reset CH32V203 (power cycle or nRST pulse) to run blink.")
        print("PA0 (pin 1) should toggle at ~1 Hz.")

    return FAIL_COUNT == 0


if __name__ == "__main__":
    success = main()
    sys.exit(0 if success else 1)
