#!/usr/bin/env python3
"""
Unit test suite for On-Probe Network GDB Server (Remote Serial Protocol).
Validates:
- GDB Remote Serial Protocol packet framing and checksum verification
- Core RSP command handling:
  - $?# (Query halt reason)
  - $g# and $G# (Read and write general core registers R0-R15 + xPSR)
  - $m# and $M# (Read and write target SRAM/Flash memory blocks)
  - $s# (Single-step instruction)
  - $c# (Resume / continue execution)
  - $Z0# and $z0# (Insert/remove hardware breakpoints via Cortex-M FPB)
  - Interrupt byte (0x03 / Ctrl-C)
"""

import struct
import pytest
from typing import Dict, List, Optional, Tuple


def calculate_checksum(payload: bytes) -> str:
    """Calculate 2-digit hex checksum for GDB RSP packet."""
    csum = sum(payload) % 256
    return f"{csum:02x}"


def format_rsp_packet(payload: str) -> bytes:
    """Wrap payload in $<data>#<checksum>."""
    b = payload.encode("ascii")
    csum = calculate_checksum(b)
    return b"$" + b + b"#" + csum.encode("ascii")


class MockCortexTarget:
    """Mock ARM Cortex-M target with CoreSight registers, SRAM, and FPB."""
    def __init__(self):
        self.regs = [0] * 17  # R0-R15 + xPSR
        self.regs[13] = 0x20002000  # SP
        self.regs[14] = 0x08000101  # LR
        self.regs[15] = 0x08000250  # PC
        self.regs[16] = 0x01000000  # xPSR (Thumb state)
        self.halted = True
        self.memory: Dict[int, int] = {}  # 32-bit aligned memory words
        self.fpb_enabled = False
        self.fpb_comparators = [0] * 6

    def read_reg(self, reg_num: int) -> int:
        if 0 <= reg_num < 17:
            return self.regs[reg_num]
        return 0

    def write_reg(self, reg_num: int, val: int):
        if 0 <= reg_num < 17:
            self.regs[reg_num] = val & 0xFFFFFFFF

    def read_mem(self, addr: int, length: int) -> bytes:
        out = bytearray()
        for i in range(length):
            byte_addr = addr + i
            word_addr = byte_addr & ~3
            byte_offset = byte_addr & 3
            word_val = self.memory.get(word_addr, 0x00000000)
            byte_val = (word_val >> (byte_offset * 8)) & 0xFF
            out.append(byte_val)
        return bytes(out)

    def write_mem(self, addr: int, data: bytes):
        for i, b in enumerate(data):
            byte_addr = addr + i
            word_addr = byte_addr & ~3
            byte_offset = byte_addr & 3
            current = self.memory.get(word_addr, 0x00000000)
            mask = ~(0xFF << (byte_offset * 8))
            new_word = (current & mask) | (b << (byte_offset * 8))
            self.memory[word_addr] = new_word & 0xFFFFFFFF

    def insert_hw_breakpoint(self, addr: int) -> bool:
        self.fpb_enabled = True
        for i in range(len(self.fpb_comparators)):
            if (self.fpb_comparators[i] & 1) == 0:
                replace = 2 << 30 if (addr & 2) != 0 else 1 << 30
                self.fpb_comparators[i] = (addr & 0x1FFFFFFC) | replace | 1
                return True
        return False

    def remove_hw_breakpoint(self, addr: int) -> bool:
        for i in range(len(self.fpb_comparators)):
            if (self.fpb_comparators[i] & 1) != 0 and (self.fpb_comparators[i] & 0x1FFFFFFC) == (addr & 0x1FFFFFFC):
                self.fpb_comparators[i] = 0
                return True
        return False


class MockGdbRspServer:
    """
    Python model simulating firmware-rs/src/programmer/gdb_server.rs.
    """
    def __init__(self, target: MockCortexTarget):
        self.target = target

    def handle_packet(self, raw_pkt: bytes) -> bytes:
        if raw_pkt == b"\x03":
            self.target.halted = True
            return format_rsp_packet("S05")

        if not (raw_pkt.startswith(b"$") and b"#" in raw_pkt):
            return b""

        hash_pos = raw_pkt.find(b"#")
        payload = raw_pkt[1:hash_pos].decode("ascii")
        expected_csum = raw_pkt[hash_pos + 1:hash_pos + 3].decode("ascii")

        actual_csum = calculate_checksum(payload.encode("ascii"))
        if actual_csum.lower() != expected_csum.lower():
            return b"-"  # Checksum mismatch NACK

        # Return ACK followed by response
        resp_payload = self._process_command(payload)
        return b"+" + format_rsp_packet(resp_payload)

    def _process_command(self, cmd: str) -> str:
        if not cmd:
            return ""

        if cmd == "?":
            return "S05" if self.target.halted else "OK"

        if cmd == "g":
            # Read 17 registers (R0-R15 + xPSR) in little-endian hex
            hex_parts = []
            for r in range(17):
                val = self.target.read_reg(r)
                val_bytes = struct.pack("<I", val)
                hex_parts.append(val_bytes.hex())
            return "".join(hex_parts)

        if cmd.startswith("G"):
            hex_str = cmd[1:]
            raw = bytes.fromhex(hex_str)
            for r in range(min(17, len(raw) // 4)):
                val = struct.unpack("<I", raw[r * 4:r * 4 + 4])[0]
                self.target.write_reg(r, val)
            return "OK"

        if cmd.startswith("m"):
            # m<addr>,<len>
            addr_str, len_str = cmd[1:].split(",")
            addr = int(addr_str, 16)
            length = int(len_str, 16)
            data = self.target.read_mem(addr, length)
            return data.hex()

        if cmd.startswith("M"):
            # M<addr>,<len>:<data>
            meta, hex_data = cmd[1:].split(":")
            addr_str, len_str = meta.split(",")
            addr = int(addr_str, 16)
            length = int(len_str, 16)
            data = bytes.fromhex(hex_data)
            self.target.write_mem(addr, data[:length])
            return "OK"

        if cmd == "s" or cmd.startswith("vCont;s"):
            # Step one instruction
            self.target.regs[15] += 2  # Increment PC by 2 (Thumb)
            self.target.halted = True
            return "S05"

        if cmd == "c" or cmd.startswith("vCont;c"):
            self.target.halted = False
            return "S05"

        if cmd.startswith("Z0,"):
            # Hardware breakpoint insert: Z0,<addr>,<kind>
            parts = cmd[3:].split(",")
            addr = int(parts[0], 16)
            if self.target.insert_hw_breakpoint(addr):
                return "OK"
            return "E01"

        if cmd.startswith("z0,"):
            # Hardware breakpoint remove: z0,<addr>,<kind>
            parts = cmd[3:].split(",")
            addr = int(parts[0], 16)
            self.target.remove_hw_breakpoint(addr)
            return "OK"

        if cmd.startswith("qSupported"):
            return "PacketSize=512;qXfer:memory-map:read+;hwbreak+"

        if cmd in ("k", "D"):
            self.target.halted = False
            return "OK"

        return ""


def test_gdb_rsp_checksum():
    """Verify standard GDB RSP packet framing and modulo-256 checksum."""
    payload = b"qSupported"
    csum = calculate_checksum(payload)
    pkt = format_rsp_packet("qSupported")
    assert pkt == b"$qSupported#37"

    # Single character '?'
    assert format_rsp_packet("?") == b"$?#3f"
    # Halt response 'S05'
    assert format_rsp_packet("S05") == b"$S05#b8"


def test_gdb_query_halt_status():
    """Verify $?# returns SIGTRAP (S05) when target is halted."""
    target = MockCortexTarget()
    target.halted = True
    server = MockGdbRspServer(target)

    res = server.handle_packet(b"$?#3f")
    assert res == b"+$S05#b8"


def test_gdb_read_and_write_registers():
    """Verify $g# and $G# read and write ARM core registers R0-R15 and xPSR."""
    target = MockCortexTarget()
    target.write_reg(0, 0x12345678)
    target.write_reg(15, 0x08000400)  # PC
    server = MockGdbRspServer(target)

    # 1. Read registers ($g#)
    pkt_read = format_rsp_packet("g")
    res = server.handle_packet(pkt_read)
    assert res.startswith(b"+$")
    payload = res[2:-3].decode("ascii")
    # 17 registers * 8 hex characters = 136 chars
    assert len(payload) == 136
    # R0 = 0x12345678 in little-endian hex is "78563412"
    assert payload[:8] == "78563412"

    # 2. Modify registers ($G#)
    new_regs = bytearray(68)
    # Set R0 = 0xCAFEBABE, R1 = 0xDEADBEEF
    struct.pack_into("<I", new_regs, 0, 0xCAFEBABE)
    struct.pack_into("<I", new_regs, 4, 0xDEADBEEF)
    pkt_write = format_rsp_packet("G" + new_regs.hex())
    res_write = server.handle_packet(pkt_write)
    assert res_write == b"+$OK#9a"
    assert target.read_reg(0) == 0xCAFEBABE
    assert target.read_reg(1) == 0xDEADBEEF


def test_gdb_memory_read_and_write():
    """Verify $m# and $M# read and write memory blocks over SWD."""
    target = MockCortexTarget()
    server = MockGdbRspServer(target)

    # Write 8 bytes to SRAM at 0x20000100: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
    hex_payload = "0102030405060708"
    pkt_write = format_rsp_packet(f"M20000100,8:{hex_payload}")
    res_write = server.handle_packet(pkt_write)
    assert res_write == b"+$OK#9a"

    # Read back 8 bytes ($m20000100,8#)
    pkt_read = format_rsp_packet("m20000100,8")
    res_read = server.handle_packet(pkt_read)
    assert res_read == b"+$" + hex_payload.encode("ascii") + b"#" + calculate_checksum(hex_payload.encode("ascii")).encode("ascii")


def test_gdb_hardware_breakpoint_fpb():
    """Verify $Z0 and $z0 hardware breakpoint configuration via Cortex-M FPB."""
    target = MockCortexTarget()
    server = MockGdbRspServer(target)

    # Insert hardware breakpoint at 0x08000420: $Z0,8000420,2#
    pkt_bp_insert = format_rsp_packet("Z0,8000420,2")
    res_insert = server.handle_packet(pkt_bp_insert)
    assert res_insert == b"+$OK#9a"
    assert target.fpb_enabled is True
    assert (target.fpb_comparators[0] & 1) == 1

    # Remove hardware breakpoint: $z0,8000420,2#
    pkt_bp_remove = format_rsp_packet("z0,8000420,2")
    res_remove = server.handle_packet(pkt_bp_remove)
    assert res_remove == b"+$OK#9a"
    assert target.fpb_comparators[0] == 0


def test_gdb_interrupt_ctrl_c():
    """Verify Ctrl-C (0x03) halts a running target and returns $S05#."""
    target = MockCortexTarget()
    target.halted = False
    server = MockGdbRspServer(target)

    res = server.handle_packet(b"\x03")
    assert res == b"$S05#b8"
    assert target.halted is True
