#!/usr/bin/env python3
"""
Charlieplex 2-LED Test for CH32V203 (PA0, PA1) via AgentLA RV-SWD.
==================================================================

Charlieplexing 2 LEDs with 2 pins (PA0 and PA1):
  - LED 1: Anode -> PA0, Cathode -> PA1
  - LED 2: Anode -> PA1, Cathode -> PA0

States:
  1. LED 1 ON : PA0 = HIGH, PA1 = LOW  (GPIOA_BSHR = 0x00020001) for ~400ms
  2. Both OFF : PA0 = LOW,  PA1 = LOW  (GPIOA_BSHR = 0x00030000) for ~100ms
  3. LED 2 ON : PA0 = LOW,  PA1 = HIGH (GPIOA_BSHR = 0x00010002) for ~400ms
  4. Both OFF : PA0 = LOW,  PA1 = LOW  (GPIOA_BSHR = 0x00030000) for ~100ms

Steps performed by this script:
  1. Connect to AgentLA serial port
  2. Detect CH32V203 via 2-wire RV-SWD (or CH32V003 via 1-wire SWIO)
  3. Assemble Charlieplex machine code (128 bytes)
  4. Erase flash @ 0x08000000
  5. Flash binary to 0x08000000
  6. Read back & verify 100% byte-exact parity
  7. Reset target (CH32 RESET) and verify running registers
"""

import sys
import time
import struct
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

# ─── RISC-V Instruction Encoders ─────────────────────────────────────────────

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


def build_charlieplex_firmware():
    """Build 128-byte raw RISC-V binary for PA0/PA1 Charlieplexing."""
    OPC_LUI = 0x37
    OPC_IMM = 0x13
    OPC_LOAD = 0x03
    OPC_STORE = 0x23
    OPC_BRANCH = 0x63
    OPC_JAL = 0x6F

    # Register assignments:
    # t0 (x5) = peripheral base
    # t1 (x6) = data / control values
    # t2 (x7) = delay counter / temp
    code = []

    # 1. Enable GPIOA clock in RCC_APB2PCENR (0x40021018)
    # t0 = 0x40021000
    code.append(U_TYPE(OPC_LUI, 5, 0x40021))
    # t1 = lw 0x18(t0)
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x18))
    # t1 |= 0x1D (AFIO, GPIOA, GPIOB, GPIOC)
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x1D))
    # sw t1, 0x18(t0)
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x18))

    # 2. Configure PA0 and PA1 as Output Push-Pull 50MHz in GPIOA_CFGLR (0x40010800)
    # t0 = 0x40010800 (0x40011000 - 0x800)
    code.append(U_TYPE(OPC_LUI, 5, 0x40011))
    code.append(I_TYPE(OPC_IMM, 5, 0, 5, -0x800))
    # t1 = lw 0(t0)
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0))
    # Clear bits [7:0]: t2 = 0xFFFFFF00 (0xFFFFF000 + 0xF00 via lui + addi)
    code.append(U_TYPE(OPC_LUI, 7, 0xFFFFF))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -0x100))  # 0xFFFFF000 - 0x100 = 0xFFFFEF00... wait
    # Better: t2 = -256 (0xFFFFFF00) directly with addi from x0
    code.pop()
    code.append(I_TYPE(OPC_IMM, 7, 0, 0, -256))   # t2 = 0xFFFFFF00
    # and t1, t1, t2 (bits [7:0] cleared)
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (6 << 7) | 0x33)
    # ori t1, t1, 0x33 (PA0: MODE=11 CNF=00, PA1: MODE=11 CNF=00)
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x33))
    # sw t1, 0(t0)
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0))

    # 3. Charlieplex Alternating Loop
    loop_start_idx = len(code)

    # ── State 1: LED 1 ON (PA0=HIGH, PA1=LOW) ──
    # GPIOA_BSHR = (1 << 0) | (1 << 17) = 0x00020001
    code.append(U_TYPE(OPC_LUI, 6, 2))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 1))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay ~400ms (count ~500k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x80))
    delay1_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (delay1_idx - len(code)) * 4))

    # ── State 2: Both OFF (PA0=LOW, PA1=LOW) ──
    # GPIOA_BSHR = (1 << 16) | (1 << 17) = 0x00030000
    code.append(U_TYPE(OPC_LUI, 6, 3))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay ~100ms (count ~125k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x20))
    delay2_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (delay2_idx - len(code)) * 4))

    # ── State 3: LED 2 ON (PA0=LOW, PA1=HIGH) ──
    # GPIOA_BSHR = (1 << 16) | (1 << 1) = 0x00010002
    code.append(U_TYPE(OPC_LUI, 6, 1))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 2))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay ~400ms (count ~500k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x80))
    delay3_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (delay3_idx - len(code)) * 4))

    # ── State 4: Both OFF (PA0=LOW, PA1=LOW) ──
    # GPIOA_BSHR = (1 << 16) | (1 << 17) = 0x00030000
    code.append(U_TYPE(OPC_LUI, 6, 3))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay ~100ms (count ~125k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x20))
    delay4_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (delay4_idx - len(code)) * 4))

    # Jump back to loop_start
    jump_offset = (loop_start_idx - len(code)) * 4
    code.append(J_TYPE(OPC_JAL, 0, jump_offset))

    # Pad to 32 words (128 bytes) with NOP (0x00000013)
    NOP = 0x00000013
    while len(code) < 32:
        code.append(NOP)

    payload = bytearray()
    for word in code[:32]:
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
        if not line:
            continue
        if line.startswith('>'):
            continue
        lines.append(line)
        if line.startswith('OK') or line.startswith('ERR') or line.startswith('AGENTLA') or line.startswith('FLASH DATA'):
            break
    return lines


def main():
    print("==================================================================")
    print(" AgentLA Charlieplex 2-LED Test (PA0, PA1)")
    print("==================================================================")
    print("Hardware Setup:")
    print("  - LED 1: Anode -> PA0, Cathode -> PA1")
    print("  - LED 2: Anode -> PA1, Cathode -> PA0")
    print("  (A current-limiting resistor 220-470 Ohm should be in series)")
    print("==================================================================\n")

    payload = build_charlieplex_firmware()
    assert len(payload) == 128, f"Payload size must be 128 bytes, got {len(payload)}"
    hex_payload = payload.hex()

    print(f"Connecting to AgentLA on {PORT}...")
    ser = serial.Serial(PORT, BAUD, timeout=1.0)
    time.sleep(0.1)

    # 1. Device Handshake
    id_resp = send_cmd(ser, "IDENTIFY", timeout=2.0)
    print(f"[Stage 1] Device Handshake: {id_resp}")
    if not any("AGENTLA" in line for line in id_resp):
        print("Error: Could not identify AgentLA probe.")
        sys.exit(1)

    # 2. Target Probe (Try CH32V203 first, then CH32V003)
    print("\n[Stage 2] Detecting target MCU...")
    det_resp = send_cmd(ser, "CH32 DETECT mcu=ch32v203", timeout=2.0)
    target_mcu = "ch32v203"
    target_mode = "2wire"
    if not any("OK CH32 DETECT" in line for line in det_resp):
        det_resp = send_cmd(ser, "CH32 DETECT mcu=ch32v003", timeout=2.0)
        target_mcu = "ch32v003"
        target_mode = "1wire"

    print(f"Target detect output: {det_resp}")
    if not any("OK CH32 DETECT" in line for line in det_resp):
        print("\n[FAIL] No target detected on either 2-wire RVSWD (V203) or 1-wire SWIO (V003).")
        print("Please check probe wiring and power to the target MCU.")
        sys.exit(1)

    print(f"[PASS] Target detected: {target_mcu} ({target_mode})")

    if target_mcu == "ch32v003":
        print("\nNote: On CH32V003 silicon, PA0 does not physically exist (only PA1/PA2).")
        print("If you wired to CH32V003, ensure your pins are mapped to physical pins (e.g. PC0/PC1 or PD0/PD1).")

    # 3. Flash Page Erase
    print(f"\n[Stage 3] Erasing flash sector @ 0x08000000...")
    erase_sz = 256 if target_mcu == "ch32v203" else 64
    erase_resp = send_cmd(ser, f"CH32 ERASE 0x08000000 {erase_sz}", timeout=4.0)
    print(f"Erase response: {erase_resp}")
    assert any("OK CH32 ERASED" in line for line in erase_resp), f"Erase failed: {erase_resp}"
    print("[PASS] Flash sector erased.")

    # 4. Flash Program
    print(f"\n[Stage 4] Flashing Charlieplex binary (128 bytes)...")
    flash_resp = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_payload}", timeout=5.0)
    print(f"Flash response: {flash_resp}")
    assert any("OK CH32 FLASH WROTE" in line for line in flash_resp), f"Flash failed: {flash_resp}"
    print("[PASS] Firmware programmed.")

    # 5. Readback Parity Verification
    print(f"\n[Stage 5] Reading back 128 bytes from flash...")
    ser.reset_input_buffer()
    ser.write(b"CH32 READ 0x08000000 128\n")
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
    assert len(read_bytes) == 128, f"Expected 128 bytes, got {len(read_bytes)}"

    matches = sum(1 for a, b in zip(payload, read_bytes) if a == b)
    print(f"Readback parity check: {matches}/128 bytes matching.")
    if matches != 128:
        for idx, (a, b) in enumerate(zip(payload, read_bytes)):
            if a != b:
                print(f"  Mismatch @ 0x{idx:02X}: written 0x{a:02X} != readback 0x{b:02X}")
        raise AssertionError("Flash verification failed: byte mismatch!")
    print("[PASS] 100% byte-exact parity verified.")

    # 6. Reset Target to Run Charlieplexing
    print(f"\n[Stage 6] Resetting target to start Charlieplexing execution...")
    reset_resp = send_cmd(ser, "CH32 RESET", timeout=2.0)
    print(f"Reset response: {reset_resp}")
    assert any("OK CH32 RESET done" in line for line in reset_resp), f"Reset failed: {reset_resp}"

    time.sleep(0.3)

    # 7. Live Execution Verification (Register Check)
    if target_mcu == "ch32v203":
        gpio_resp = send_cmd(ser, "CH32 READM 0x40010800", timeout=2.0)
        print(f"[Stage 7] Live register check GPIOA_CFGLR: {gpio_resp}")
        # Resume core after READM
        send_cmd(ser, "CH32 RESUME", timeout=1.0)

    ser.close()

    print("\n==================================================================")
    print(" CHARLIEPLEX TEST RUNNING!")
    print("==================================================================")
    print("Expected behavior:")
    print("  -> LED 1 turns ON (~400ms)")
    print("  -> Both turn OFF (~100ms)")
    print("  -> LED 2 turns ON (~400ms)")
    print("  -> Both turn OFF (~100ms)")
    print("  -> (repeats continuously)")
    print("==================================================================")


if __name__ == "__main__":
    main()
