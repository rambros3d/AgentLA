#!/usr/bin/env python3
"""
Hardware Charlieplex Test for CH32V003 via AgentLA 1-Wire SWIO.
================================================================

Drives Charlieplex 2-LED pairs on:
  1. Port A: PA1 (Pin 5) & PA2 (Pin 6)
  2. Port C: PC0 (Pin 10) & PC1 (Pin 11)

(Both pairs are driven synchronously so whether you wired to
 Port A pins 5/6 or Port C pins 10/11, the LEDs will Charlieplex!)

Circuit (for each pair):
  - LED 1: Anode -> Pin A (PA1 or PC0), Cathode -> Pin B (PA2 or PC1)
  - LED 2: Anode -> Pin B (PA2 or PC1), Cathode -> Pin A (PA1 or PC0)
  - Series current-limiting resistor (220 - 470 Ohm).

Cycles through 4 states:
  State 1: LED 1 ON  (Pin A = HIGH, Pin B = LOW)  for ~400ms
  State 2: Both OFF  (Pin A = LOW,  Pin B = LOW)  for ~100ms
  State 3: LED 2 ON  (Pin A = LOW,  Pin B = HIGH) for ~400ms
  State 4: Both OFF  (Pin A = LOW,  Pin B = LOW)  for ~100ms
"""

import sys
import time
import struct
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

# ─── RISC-V 32-bit Instruction Encoders ───────────────────────────────────────

def U_TYPE(opcode, rd, imm20):
    return ((imm20 & 0xFFFFF) << 12) | ((rd & 0x1F) << 7) | (opcode & 0x7F)

def I_TYPE(opcode, rd, funct3, rs1, imm12):
    return ((imm12 & 0xFFF) << 20) | ((rs1 & 0x1F) << 15) | ((funct3 & 7) << 12) | ((rd & 0x1F) << 7) | (opcode & 0x7F)

def S_TYPE(opcode, funct3, rs1, rs2, imm12):
    imm12 = imm12 & 0xFFF
    imm_hi = (imm12 >> 5) & 0x7F
    imm_lo = imm12 & 0x1F
    return (imm_hi << 25) | ((rs2 & 0x1F) << 20) | ((rs1 & 0x1F) << 15) | ((funct3 & 7) << 12) | (imm_lo << 7) | (opcode & 0x7F)

def B_TYPE(opcode, funct3, rs1, rs2, imm13):
    imm13 = imm13 & 0x1FFE
    b12 = (imm13 >> 12) & 1
    b11 = (imm13 >> 11) & 1
    b10_5 = (imm13 >> 5) & 0x3F
    b4_1 = (imm13 >> 1) & 0xF
    return (b12 << 31) | (b10_5 << 25) | ((rs2 & 0x1F) << 20) | ((rs1 & 0x1F) << 15) | ((funct3 & 7) << 12) | (b4_1 << 8) | (b11 << 7) | (opcode & 0x7F)

def J_TYPE(opcode, rd, imm21):
    imm21 = imm21 & 0x1FFFFE
    b20 = (imm21 >> 20) & 1
    b19_12 = (imm21 >> 12) & 0xFF
    b11 = (imm21 >> 11) & 1
    b10_1 = (imm21 >> 1) & 0x3FF
    return (b20 << 31) | (b10_1 << 21) | (b11 << 20) | (b19_12 << 12) | ((rd & 0x1F) << 7) | (opcode & 0x7F)


def build_v003_charlieplex_firmware():
    """Build RISC-V firmware for CH32V003 Charlieplexing."""
    OPC_LUI = 0x37
    OPC_IMM = 0x13
    OPC_LOAD = 0x03
    OPC_STORE = 0x23
    OPC_BRANCH = 0x63
    OPC_JAL = 0x6F

    # Register usage:
    # x5 (t0) = RCC / AFIO base
    # x6 (t1) = temp / data
    # x7 (t2) = delay counter / temp
    # x8 (s0) = GPIOA base (0x40010800)
    # x9 (s1) = GPIOC base (0x40011000)

    code = []

    # 1. Enable Clocks in RCC_APB2PCENR (0x40021018):
    # AFIOEN (bit 0), IOPAEN (bit 2), IOPCEN (bit 4) -> 0x15
    code.append(U_TYPE(OPC_LUI, 5, 0x40021))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x18))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x15))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x18))

    # 2. Release PA1 & PA2 from crystal oscillator via AFIO_PCFR1 (0x40010004)
    # AFIO base = 0x40010000
    code.append(U_TYPE(OPC_LUI, 5, 0x40010))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x04))
    # Clear bit 15: t2 = 0xFFFF7FFF
    code.append(U_TYPE(OPC_LUI, 7, 0xFFFF8))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (6 << 7) | 0x33)  # and t1, t1, t2
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x04))

    # 3. Configure GPIOA (0x40010800): PA1 & PA2 as Output Push-Pull 50MHz (MODE=11, CNF=00 -> 0x3)
    # s0 (x8) = 0x40010800
    code.append(U_TYPE(OPC_LUI, 8, 0x40011))
    code.append(I_TYPE(OPC_IMM, 8, 0, 8, -0x800))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 8, 0))
    # Clear bits [11:4] in GPIOA_CFGLR
    code.append(U_TYPE(OPC_LUI, 7, 0xFFFFF))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, 0x00F))  # t2 = 0xFFFFF00F
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (6 << 7) | 0x33)
    # Set bits [11:4] = 0x330 (PA1=3, PA2=3)
    code.append(I_TYPE(OPC_IMM, 7, 0, 0, 0x330))
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (6 << 12) | (6 << 7) | 0x33)  # or t1, t1, t2
    code.append(S_TYPE(OPC_STORE, 2, 8, 6, 0))

    # 4. Configure GPIOC (0x40011000): PC0 & PC1 as Output Push-Pull 50MHz
    # s1 (x9) = 0x40011000
    code.append(U_TYPE(OPC_LUI, 9, 0x40011))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 9, 0))
    # Clear bits [7:0]
    code.append(I_TYPE(OPC_IMM, 7, 0, 0, -256))   # 0xFFFFFF00
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (6 << 7) | 0x33)
    # Set bits [7:0] = 0x33
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x33))
    code.append(S_TYPE(OPC_STORE, 2, 9, 6, 0))

    # 5. Charlieplex Alternating Loop
    loop_start_idx = len(code)

    # ── State 1: LED 1 ON ──
    # GPIOA: PA1=HIGH, PA2=LOW -> (1<<1)|(1<<18) = 0x00040002
    code.append(U_TYPE(OPC_LUI, 6, 4))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 2))
    code.append(S_TYPE(OPC_STORE, 2, 8, 6, 0x10))
    # GPIOC: PC0=HIGH, PC1=LOW -> (1<<0)|(1<<17) = 0x00020001
    code.append(U_TYPE(OPC_LUI, 6, 2))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 1))
    code.append(S_TYPE(OPC_STORE, 2, 9, 6, 0x10))

    # Delay ~400ms (count ~300k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x48))
    d1 = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d1 - len(code)) * 4))

    # ── State 2: Both OFF ──
    # GPIOA: PA1=LOW, PA2=LOW -> (1<<17)|(1<<18) = 0x00060000
    code.append(U_TYPE(OPC_LUI, 6, 6))
    code.append(S_TYPE(OPC_STORE, 2, 8, 6, 0x10))
    # GPIOC: PC0=LOW, PC1=LOW -> (1<<16)|(1<<17) = 0x00030000
    code.append(U_TYPE(OPC_LUI, 6, 3))
    code.append(S_TYPE(OPC_STORE, 2, 9, 6, 0x10))

    # Delay ~100ms
    code.append(U_TYPE(OPC_LUI, 7, 0x12))
    d2 = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d2 - len(code)) * 4))

    # ── State 3: LED 2 ON ──
    # GPIOA: PA1=LOW, PA2=HIGH -> (1<<17)|(1<<2) = 0x00020004
    code.append(U_TYPE(OPC_LUI, 6, 2))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 4))
    code.append(S_TYPE(OPC_STORE, 2, 8, 6, 0x10))
    # GPIOC: PC0=LOW, PC1=HIGH -> (1<<16)|(1<<1) = 0x00010002
    code.append(U_TYPE(OPC_LUI, 6, 1))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 2))
    code.append(S_TYPE(OPC_STORE, 2, 9, 6, 0x10))

    # Delay ~400ms
    code.append(U_TYPE(OPC_LUI, 7, 0x48))
    d3 = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d3 - len(code)) * 4))

    # ── State 4: Both OFF ──
    # GPIOA: PA1=LOW, PA2=LOW -> 0x00060000
    code.append(U_TYPE(OPC_LUI, 6, 6))
    code.append(S_TYPE(OPC_STORE, 2, 8, 6, 0x10))
    # GPIOC: PC0=LOW, PC1=LOW -> 0x00030000
    code.append(U_TYPE(OPC_LUI, 6, 3))
    code.append(S_TYPE(OPC_STORE, 2, 9, 6, 0x10))

    # Delay ~100ms
    code.append(U_TYPE(OPC_LUI, 7, 0x12))
    d4 = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d4 - len(code)) * 4))

    # Jump back to loop_start
    jump_offset = (loop_start_idx - len(code)) * 4
    code.append(J_TYPE(OPC_JAL, 0, jump_offset))

    # Pad to 64-byte page boundary (16 words per page)
    NOP = 0x00000013
    page_words = 16
    while len(code) % page_words != 0:
        code.append(NOP)

    payload = bytearray()
    for word in code:
        payload += struct.pack('<I', word & 0xFFFFFFFF)

    return payload


def send_cmd(ser, cmd, timeout=3.0):
    ser.reset_input_buffer()
    ser.write(f"{cmd}\n".encode())
    time.sleep(0.05)
    t0 = time.time()
    lines = []
    while time.time() - t0 < timeout:
        line = ser.readline().decode('utf-8', errors='replace').strip()
        if not line or line.startswith('>'):
            continue
        lines.append(line)
        if line.startswith('OK') or line.startswith('ERR') or line.startswith('AGENTLA') or line.startswith('FLASH DATA'):
            break
    return lines


def main():
    print("==================================================================")
    print(" CH32V003 Charlieplex 2-LED Hardware Verification (AgentLA SWIO)")
    print("==================================================================")
    print("Synchronous Pin Outputs:")
    print("  • Port A: PA1 (Pin 5) & PA2 (Pin 6)")
    print("  • Port C: PC0 (Pin 10) & PC1 (Pin 11)")
    print("  (Both pairs toggle identically so either wiring works!)")
    print("==================================================================\n")

    payload = build_v003_charlieplex_firmware()
    payload_len = len(payload)
    print(f"Compiled binary: {payload_len} bytes ({payload_len // 64} pages).")
    hex_payload = payload.hex()

    print(f"Connecting to AgentLA on {PORT}...")
    ser = serial.Serial(PORT, BAUD, timeout=1.0)
    time.sleep(0.1)

    # 1. Device Handshake
    id_resp = send_cmd(ser, "IDENTIFY", timeout=2.0)
    print(f"[Stage 1] AgentLA Handshake: {id_resp}")
    if not any("AGENTLA" in line for line in id_resp):
        print("Error: Could not identify AgentLA probe.")
        sys.exit(1)

    # 2. CH32V003 Detection
    print("\n[Stage 2] Detecting CH32V003 over 1-Wire SWIO...")
    det_resp = send_cmd(ser, "CH32 DETECT mcu=ch32v003", timeout=3.0)
    print(f"Target detect: {det_resp}")
    if not any("OK CH32 DETECT" in line for line in det_resp):
        print("\n[FAIL] CH32V003 target not detected. Check SWIO wiring & pullup.")
        sys.exit(1)
    print("[PASS] CH32V003 detected successfully!")

    # 3. Flash Page Erase
    print(f"\n[Stage 3] Erasing {payload_len} bytes @ 0x08000000...")
    erase_resp = send_cmd(ser, f"CH32 ERASE 0x08000000 {payload_len}", timeout=4.0)
    print(f"Erase response: {erase_resp}")
    assert any("OK CH32 ERASED" in line for line in erase_resp), f"Erase failed: {erase_resp}"
    print("[PASS] Flash erased.")

    # 4. Flash Program
    print(f"\n[Stage 4] Flashing Charlieplex binary ({payload_len} bytes)...")
    flash_resp = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_payload}", timeout=6.0)
    print(f"Flash response: {flash_resp}")
    assert any("OK CH32 FLASH WROTE" in line for line in flash_resp), f"Flash failed: {flash_resp}"
    print("[PASS] Flash programming successful.")

    # 5. Readback Parity Verification
    print(f"\n[Stage 5] Reading back {payload_len} bytes from 0x08000000...")
    ser.reset_input_buffer()
    ser.write(f"CH32 READ 0x08000000 {payload_len}\n".encode())
    t0 = time.time()
    data_line = None
    read_resp = []
    while time.time() - t0 < 6.0:
        line = ser.readline().decode('utf-8', errors='replace').strip()
        if not line or line.startswith('>'):
            continue
        read_resp.append(line)
        if line.startswith("FLASH DATA"):
            data_line = line
        if line.startswith("OK CH32 READ") or line.startswith("ERR"):
            break

    assert data_line is not None, f"No FLASH DATA received: {read_resp}"
    bytes_csv = data_line.replace("FLASH DATA", "").strip()
    read_bytes = bytearray(int(b, 16) for b in bytes_csv.split(",") if b)
    print(f"Received {len(read_bytes)} bytes from flash readback.")
    assert len(read_bytes) == payload_len, f"Expected {payload_len} bytes, got {len(read_bytes)}"

    matches = sum(1 for a, b in zip(payload, read_bytes) if a == b)
    print(f"Readback parity check: {matches}/{payload_len} bytes matching.")
    if matches != payload_len:
        for idx, (a, b) in enumerate(zip(payload, read_bytes)):
            if a != b:
                print(f"  Mismatch @ 0x{idx:02X}: written 0x{a:02X} != readback 0x{b:02X}")
        raise AssertionError("Flash verification failed: byte mismatch!")
    print("[PASS] 100% byte-exact parity verified.")

    # 6. Reset Target to Run Charlieplex Firmware
    print("\n[Stage 6] Resetting target to start Charlieplex execution...")
    reset_resp = send_cmd(ser, "CH32 RESET", timeout=2.0)
    print(f"Reset response: {reset_resp}")
    assert any("OK CH32 RESET done" in line for line in reset_resp), f"Reset failed: {reset_resp}"

    time.sleep(0.3)

    ser.close()

    print("\n" + "=" * 66)
    print(" >>> CH32V003 CHARLIEPLEX FIRMWARE EXECUTING LIVE! <<<")
    print("=" * 66)
    print("Visual Inspection:")
    print("  • LED 1 turns ON  (~400ms)")
    print("  • Both turn OFF   (~100ms)")
    print("  • LED 2 turns ON  (~400ms)")
    print("  • Both turn OFF   (~100ms)")
    print("  (Repeating continuously in infinite loop)")
    print("=" * 66)


if __name__ == "__main__":
    main()
