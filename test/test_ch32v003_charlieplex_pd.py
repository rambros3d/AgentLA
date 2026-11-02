#!/usr/bin/env python3
"""
Hardware Charlieplex Test for CH32V003: PD2, PD3, PD4, PD5
============================================================
Communicates via AgentLA 1-Wire SWIO (PD1) to flash and run
the Charlieplex test firmware.

Pins Driven:
  • Pair 1: PD2 (Pin 19) & PD3 (Pin 20)
  • Pair 2: PD4 (Pin 1)  & PD5 (Pin 2)

*NOTE*: PD1 is the 1-wire SWIO debug pin and is carefully preserved
        in GPIOD_CFGLR so the debug link is never interrupted!

Cycles through 4 Charlieplex phases:
  Phase 1: PD2 = HIGH, PD3 = LOW (PD4=LOW, PD5=LOW) -> Pair 1 Forward  (~350ms)
           Both OFF                                  -> Pause           (~80ms)
  Phase 2: PD2 = LOW,  PD3 = HIGH (PD4=LOW, PD5=LOW) -> Pair 1 Reverse  (~350ms)
           Both OFF                                  -> Pause           (~80ms)
  Phase 3: PD4 = HIGH, PD5 = LOW  (PD2=LOW, PD3=LOW) -> Pair 2 Forward  (~350ms)
           Both OFF                                  -> Pause           (~80ms)
  Phase 4: PD4 = LOW,  PD5 = HIGH (PD2=LOW, PD3=LOW) -> Pair 2 Reverse  (~350ms)
           Both OFF                                  -> Pause           (~80ms)
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


def build_firmware():
    """
    Build compact 256-byte RISC-V (RV32EC compliant) firmware
    for exhaustive Charlieplex testing across PD2, PD3, PD4, PD5 (12 combinations).
    """
    OPC_LUI = 0x37
    OPC_AUIPC = 0x17
    OPC_IMM = 0x13
    OPC_LOAD = 0x03
    OPC_STORE = 0x23
    OPC_BRANCH = 0x63
    OPC_JAL = 0x6F

    # Charlieplex 12-pair permutation: all pairs (A, B) where A != B
    pairs = [
        (2, 3), (3, 2),
        (2, 4), (4, 2),
        (2, 5), (5, 2),
        (3, 4), (4, 3),
        (3, 5), (5, 3),
        (4, 5), (5, 4)
    ]

    def make_cfglr(a, b):
        # Base: 0x44444444 (all pins floating input)
        # Pins a and b: set to 0x3 (Output Push-Pull 50MHz)
        # Pin 1 (SWIO) remains 0x4 (floating input / unaltered)
        val = 0x44444444
        val &= ~(0xF << (a * 4))
        val &= ~(0xF << (b * 4))
        val |= (0x3 << (a * 4))
        val |= (0x3 << (b * 4))
        return val

    def make_bshr(a, b):
        # Lower 16 bits: Set pin A HIGH
        # Upper 16 bits: Reset pin B LOW (and also reset other PD2..PD5 pins)
        reset_mask = 0x3C & ~(1 << a)
        return (1 << a) | (reset_mask << 16)

    table = []
    for a, b in pairs:
        table.append(make_cfglr(a, b))
        table.append(make_bshr(a, b))

    code = []

    # 1. Enable RCC Clocks: RCC_APB2PCENR (0x40021018)
    # Enable AFIO (bit 0), GPIOA (bit 2), GPIOC (bit 4), GPIOD (bit 5) -> 0x35
    # x5 (t0) = RCC base (0x40021000)
    code.append(U_TYPE(OPC_LUI, 5, 0x40021))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x18))
    code.append(I_TYPE(OPC_IMM, 6, 6, 6, 0x35))
    code.append(S_TYPE(OPC_STORE, 2, 5, 6, 0x18))
    code.append(I_TYPE(OPC_LOAD, 6, 2, 5, 0x18))  # sync read

    # 2. Setup GPIOD base: x8 (s0) = 0x40011400
    code.append(U_TYPE(OPC_LUI, 8, 0x40011))
    code.append(I_TYPE(OPC_IMM, 8, 0, 8, 0x400))

    # 3. Outer Loop: loop_charlie
    loop_charlie_idx = len(code)
    auipc_idx = len(code)
    code.append(0)  # placeholder for auipc a0, 0
    code.append(0)  # placeholder for addi a0, a0, table_offset
    code.append(I_TYPE(OPC_IMM, 11, 0, 0, len(pairs)))  # a1 (x11) = 12 steps

    # 4. Step Loop: step_loop
    step_loop_idx = len(code)
    # Load CFGLR & write
    code.append(I_TYPE(OPC_LOAD, 12, 2, 10, 0))     # lw a2, 0(a0)
    code.append(S_TYPE(OPC_STORE, 2, 8, 12, 0))      # sw a2, 0(s0)
    # Load BSHR & write
    code.append(I_TYPE(OPC_LOAD, 12, 2, 10, 4))     # lw a2, 4(a0)
    code.append(S_TYPE(OPC_STORE, 2, 8, 12, 0x10))   # sw a2, 16(s0)

    # Delay ON (~350ms)
    code.append(U_TYPE(OPC_LUI, 7, 0x48))           # lui t2, 0x48
    d_on_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))        # addi t2, t2, -1
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d_on_idx - len(code)) * 4))  # bne t2, zero

    # Turn OFF: all pins floating inputs (Hi-Z), outputs reset to 0
    code.append(U_TYPE(OPC_LUI, 12, 0x44444))
    code.append(I_TYPE(OPC_IMM, 12, 6, 12, 0x444))  # ori a2, a2, 0x444 -> 0x44444444
    code.append(S_TYPE(OPC_STORE, 2, 8, 12, 0))      # sw a2, 0(s0)
    code.append(U_TYPE(OPC_LUI, 12, 0x3C0))         # a2 = 0x003C0000 (reset PD2..PD5)
    code.append(S_TYPE(OPC_STORE, 2, 8, 12, 0x10))   # sw a2, 16(s0)

    # Delay OFF (~80ms inter-step pause)
    code.append(U_TYPE(OPC_LUI, 7, 0x12))           # lui t2, 0x12
    d_off_idx = len(code)
    code.append(I_TYPE(OPC_IMM, 7, 0, 7, -1))        # addi t2, t2, -1
    code.append(B_TYPE(OPC_BRANCH, 1, 7, 0, (d_off_idx - len(code)) * 4))  # bne t2, zero

    # Advance table pointer to next (CFGLR, BSHR) entry (8 bytes)
    code.append(I_TYPE(OPC_IMM, 10, 0, 10, 8))      # addi a0, a0, 8
    # Decrement step counter
    code.append(I_TYPE(OPC_IMM, 11, 0, 11, -1))     # addi a1, a1, -1
    # Loop back if a1 != 0
    code.append(B_TYPE(OPC_BRANCH, 1, 11, 0, (step_loop_idx - len(code)) * 4))  # bne a1, zero

    # Repeat outer loop forever
    code.append(J_TYPE(OPC_JAL, 0, (loop_charlie_idx - len(code)) * 4))  # jal zero, loop_charlie

    # 5. Emit table
    table_idx = len(code)
    table_byte_offset = (table_idx - auipc_idx) * 4
    code[auipc_idx] = U_TYPE(OPC_AUIPC, 10, 0)
    code[auipc_idx + 1] = I_TYPE(OPC_IMM, 10, 0, 10, table_byte_offset)

    for word in table:
        code.append(word)

    # Pad to exactly 64 words (256 bytes)
    NOP = 0x00000013
    assert len(code) <= 64, f"Code too large: {len(code)} words > 64"
    while len(code) < 64:
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
    print(" CH32V003 Charlieplex Test: PD2, PD3, PD4, PD5 (AgentLA SWIO)")
    print("==================================================================")
    print("Active Pins:")
    print("  • Pair 1: PD2 (Pin 19) & PD3 (Pin 20)")
    print("  • Pair 2: PD4 (Pin 1)  & PD5 (Pin 2)")
    print("  • SWIO:   PD1 (Pin 8)  — Preserved & Safe")
    print("==================================================================\n")

    payload = build_firmware()
    payload_len = len(payload)
    assert payload_len == 256, f"Expected 256 bytes, got {payload_len}"
    print(f"Compiled binary: {payload_len} bytes (4 pages of 64B).")
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

    # 3. Flash Page Erase (256 bytes)
    print(f"\n[Stage 3] Erasing {payload_len} bytes @ 0x08000000...")
    erase_resp = send_cmd(ser, f"CH32 ERASE 0x08000000 {payload_len}", timeout=4.0)
    print(f"Erase response: {erase_resp}")
    assert any("OK CH32 ERASED" in line for line in erase_resp), f"Erase failed: {erase_resp}"
    print("[PASS] Flash erased.")

    # 4. Flash Program
    print(f"\n[Stage 4] Flashing Charlieplex firmware ({payload_len} bytes)...")
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
    print(" >>> PD2, PD3, PD4, PD5 FULL CHARLIEPLEX RUNNING LIVE! <<<")
    print("=" * 66)
    print("Exhaustive Visual Inspection Sequence (12 States):")
    print("  State  1: PD2(+) PD3(-) -> Anode PD2, Cathode PD3")
    print("  State  2: PD3(+) PD2(-) -> Anode PD3, Cathode PD2")
    print("  State  3: PD2(+) PD4(-) -> Anode PD2, Cathode PD4")
    print("  State  4: PD4(+) PD2(-) -> Anode PD4, Cathode PD2")
    print("  State  5: PD2(+) PD5(-) -> Anode PD2, Cathode PD5")
    print("  State  6: PD5(+) PD2(-) -> Anode PD5, Cathode PD2")
    print("  State  7: PD3(+) PD4(-) -> Anode PD3, Cathode PD4")
    print("  State  8: PD4(+) PD3(-) -> Anode PD4, Cathode PD3")
    print("  State  9: PD3(+) PD5(-) -> Anode PD3, Cathode PD5")
    print("  State 10: PD5(+) PD3(-) -> Anode PD5, Cathode PD3")
    print("  State 11: PD4(+) PD5(-) -> Anode PD4, Cathode PD5")
    print("  State 12: PD5(+) PD4(-) -> Anode PD5, Cathode PD4")
    print("  (Each state stays ON ~350ms, then OFF ~80ms before next state)")
    print("  (Repeating continuously in infinite loop)")
    print("=" * 66)


if __name__ == "__main__":
    main()
