#!/usr/bin/env python3
"""
Hardware Verification: CH32V203 PB2 Blink Test over AgentLA RV-SWD.

Steps:
  1. Handshake & Detect CH32V203 via 2-wire RV-SWD
  2. Assemble PB2 Blink Binary (92 bytes code + NOP padding = 128 bytes)
  3. Erase sector 0x08000000 (128 bytes)
  4. Flash PB2 Blink Binary to 0x08000000
  5. Read back 128 bytes from 0x08000000 and verify 100% byte parity
  6. Reset target (hardware NRST + software ndmreset via RV-SWD)
  7. Verify peripheral registers (RCC APB2 = 0x1D, GPIOB_CFGLR = 0x44444344) confirming live execution
"""

import sys
import time
import struct
import serial

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
BAUD = 115200

# ─── RISC-V Instruction Encoding ─────────────────────────────────────────────

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

def build_pb2_blink():
    OPC_LUI = 0x37
    OPC_IMM = 0x13
    OPC_LOAD = 0x03
    OPC_STORE = 0x23
    OPC_BRANCH = 0x63
    OPC_JAL = 0x6F

    code = []

    # 1. Enable GPIOB clock in RCC_APB2PCENR (0x40021018)
    # t0 = 0x40021000
    code.append(U_TYPE(OPC_LUI, 5, 0x40021))
    # t1 = lw 0x18(t0)
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x18))
    # t1 |= 0x1D (AFIO, GPIOA, GPIOB, GPIOC)
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x1D))
    # sw t1, 0x18(t0)
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x18))

    # 2. Configure PB2 as Push-Pull Output 50MHz in GPIOB_CFGLR (0x40010C00)
    # t0 = 0x40010C00 (0x40011000 - 0x400)
    code.append(U_TYPE(OPC_LUI, 5, 0x40011))
    code.append(I_TYPE(OPC_IMM, 5, 0, 5, -0x400))
    # t1 = lw 0(t0)
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0))
    # t2 = 0xFFFFF0FF
    code.append(U_TYPE(OPC_LUI, 7, 0xFFFFF))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, 0x0FF))
    # and t1, t1, t2
    code.append((0 << 25) | (7 << 20) | (6 << 15) | (7 << 12) | (6 << 7) | 0x33)
    # ori t1, t1, 0x300 (PB2: MODE=11, CNF=00 -> 0x3 in bits [11:8])
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x300))
    # sw t1, 0(t0)
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0))

    # 3. Blink loop
    loop_start_idx = len(code)
    # Set PB2 HIGH: write 0x04 to GPIOB_BSHR (0x10(t0))
    code.append(I_TYPE(OPC_IMM, 6, 0, 0, 4))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay 1 (~500k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x80))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, -4))

    # Set PB2 LOW: write 0x40000 (bit 18) to GPIOB_BSHR (0x10(t0))
    code.append(U_TYPE(OPC_LUI, 6, 0x40))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x10))

    # Delay 2 (~500k cycles)
    code.append(U_TYPE(OPC_LUI, 7, 0x80))
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, -4))

    # Jump back to loop_start
    jump_offset = (loop_start_idx - len(code)) * 4
    code.append(J_TYPE(OPC_JAL, 0, jump_offset))

    # Pad to 32 words (128 bytes) with NOP (addi x0, x0, 0 = 0x00000013)
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
    print("=== AgentLA CH32V203 PB2 Blink Flashing & Parity Verification ===")
    payload = build_pb2_blink()
    assert len(payload) == 128, f"Payload length {len(payload)} != 128"
    hex_payload = payload.hex()
    print(f"Generated 128-byte PB2 blink binary ({len(hex_payload)} hex characters).")

    print(f"Connecting to {PORT}...")
    ser = serial.Serial(PORT, BAUD, timeout=1.0)
    time.sleep(0.1)

    # 1. Identify
    id_resp = send_cmd(ser, "IDENTIFY", timeout=2.0)
    print(f"[Stage 1] IDENTIFY: {id_resp}")
    assert any("AGENTLA" in line for line in id_resp), "AgentLA identify failed"

    # 2. Detect CH32V203
    det_resp = send_cmd(ser, "CH32 DETECT mode=2wire", timeout=2.0)
    print(f"[Stage 2] CH32 DETECT: {det_resp}")
    assert any("OK CH32 DETECT" in line for line in det_resp), f"Detect failed: {det_resp}"

    # 3. Erase sector 0x08000000
    erase_resp = send_cmd(ser, "CH32 ERASE 0x08000000 128", timeout=4.0)
    print(f"[Stage 3] CH32 ERASE: {erase_resp}")
    assert any("OK CH32 ERASED" in line for line in erase_resp), f"Erase failed: {erase_resp}"

    # 4. Flash 128 bytes
    flash_resp = send_cmd(ser, f"CH32 FLASH 0x08000000 {hex_payload}", timeout=5.0)
    print(f"[Stage 4] CH32 FLASH: {flash_resp}")
    assert any("OK CH32 FLASH WROTE" in line for line in flash_resp), f"Flash failed: {flash_resp}"

    # Allow flash controller to settle
    time.sleep(0.2)

    # 5. Read back 128 bytes
    print("[Stage 5] Reading back 128 bytes...")
    ser.reset_input_buffer()
    ser.write(b"CH32 READ 0x08000000 128\n")
    data_line = None
    t0 = time.time()
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

    print(f"Read response: {read_resp}")
    assert data_line is not None, f"No FLASH DATA received: {read_resp}"

    bytes_csv = data_line.replace("FLASH DATA", "").strip()
    read_bytes = bytearray(int(b, 16) for b in bytes_csv.split(",") if b)
    print(f"Received {len(read_bytes)} bytes from flash readback.")
    assert len(read_bytes) == 128, f"Expected 128 bytes, got {len(read_bytes)}"

    # Parity check
    matches = sum(1 for a, b in zip(payload, read_bytes) if a == b)
    print(f"Byte parity verification: {matches}/128 matching.")
    if matches != 128:
        for idx, (a, b) in enumerate(zip(payload, read_bytes)):
            if a != b:
                print(f"  Mismatch at offset 0x{idx:02X}: written 0x{a:02X} != readback 0x{b:02X}")
        raise AssertionError("Flash byte parity mismatch!")

    print("[SUCCESS] Exact 128/128 byte parity verified!")

    # 6. Reset target and let it run
    reset_resp = send_cmd(ser, "CH32 RESET", timeout=2.0)
    print(f"[Stage 6] CH32 RESET: {reset_resp}")
    assert any("OK CH32 RESET done" in line for line in reset_resp), f"Reset failed: {reset_resp}"

    # Wait 300ms for blink code to initialize registers
    time.sleep(0.3)

    # 7. Verify live target execution via peripheral register check
    rcc_resp = send_cmd(ser, "CH32 READM 0x40021018", timeout=2.0)
    print(f"[Stage 7] RCC_APB2PCENR (clock enable): {rcc_resp}")
    assert any("0x0000001D" in line for line in rcc_resp), f"RCC clock not set: {rcc_resp}"

    gpio_resp = send_cmd(ser, "CH32 READM 0x40010C00", timeout=2.0)
    print(f"[Stage 7] GPIOB_CFGLR (PB2 mode): {gpio_resp}")
    assert any("0x44444344" in line for line in gpio_resp), f"GPIOB_CFGLR not configured: {gpio_resp}"

    # Resume core if READM left it paused
    send_cmd(ser, "CH32 RESUME", timeout=1.0)

    ser.close()
    print("\n===============================================================================")
    print(">>> VERIFICATION COMPLETE: PB2 Blink Firmware Flashed & Executing on CH32V203 <<<")
    print("===============================================================================")


if __name__ == "__main__":
    main()
