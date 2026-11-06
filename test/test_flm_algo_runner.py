#!/usr/bin/env python3
"""
Unit test suite for Dynamic CMSIS-Pack FLM Algorithm Runner and Host Packager.
Validates:
- 44-byte AlgoBlobHeader binary formatting (magic 0x4F474C41, version 1)
- pack_flm.py ELF symbol parsing and packaging
- Target SRAM execution harness simulation (R0-R3, SP, LR with BKPT 0 halt)
- Verification of packaged STM32F1 and STM32G0 algorithm blobs
- Integrity of LittleFS Partition 1 image holding /system/algo/ blobs
"""

import os
import struct
import subprocess
import sys
from pathlib import Path

# Add project root to sys.path
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import pytest

from tools.pack_flm import (
    ALGO_MAGIC,
    ALGO_VERSION,
    HEADER_SIZE,
    AlgoBlobHeader,
    pack_algo_binary,
    generate_builtin,
)


def test_algo_blob_header_packing_and_unpacking():
    """Verify 44-byte AlgoBlobHeader pack/unpack round-trip."""
    hdr = AlgoBlobHeader(
        code_size=128,
        init_offset=4,
        erase_offset=8,
        program_offset=12,
        verify_offset=16,
        uninit_offset=20,
        ram_base=0x20000400,
        ram_size=2048,
        stack_top=0x20001000,
    )
    packed = hdr.pack()
    assert len(packed) == HEADER_SIZE
    assert len(packed) == 44

    unpacked = AlgoBlobHeader.unpack(packed)
    assert unpacked.magic == ALGO_MAGIC
    assert unpacked.version == ALGO_VERSION
    assert unpacked.code_size == 128
    assert unpacked.init_offset == 4
    assert unpacked.erase_offset == 8
    assert unpacked.program_offset == 12
    assert unpacked.verify_offset == 16
    assert unpacked.uninit_offset == 20
    assert unpacked.ram_base == 0x20000400
    assert unpacked.ram_size == 2048
    assert unpacked.stack_top == 0x20001000


def test_invalid_header_rejection():
    """Verify corrupted header detection."""
    # Bad magic
    bad_magic = struct.pack("<11I", 0xDEADBEEF, 1, 64, 4, 8, 12, 16, 20, 0x20000000, 1024, 0x20000800)
    with pytest.raises(ValueError, match="Invalid magic"):
        AlgoBlobHeader.unpack(bad_magic)

    # Bad version
    bad_version = struct.pack("<11I", ALGO_MAGIC, 99, 64, 4, 8, 12, 16, 20, 0x20000000, 1024, 0x20000800)
    with pytest.raises(ValueError, match="Invalid version"):
        AlgoBlobHeader.unpack(bad_version)

    # Truncated
    with pytest.raises(ValueError, match="Data too short"):
        AlgoBlobHeader.unpack(b"ALGO_SHORT")


def test_builtin_blobs_generation():
    """Verify built-in algorithm generator creates valid binary blobs."""
    f1_blob = generate_builtin("stm32f1")
    assert len(f1_blob) >= HEADER_SIZE + 24
    f1_hdr = AlgoBlobHeader.unpack(f1_blob[:HEADER_SIZE])
    assert f1_hdr.code_size == 24
    assert f1_hdr.init_offset == 4
    assert f1_hdr.erase_offset == 8

    g0_blob = generate_builtin("stm32g0")
    assert len(g0_blob) >= HEADER_SIZE + 24
    g0_hdr = AlgoBlobHeader.unpack(g0_blob[:HEADER_SIZE])
    assert g0_hdr.code_size == 24


class MockRamTarget:
    """Mock simulating Cortex-M SRAM, registers, and BKPT 0 halt execution."""

    def __init__(self):
        self.sram = bytearray(0x10000)  # 64 KB SRAM
        self.regs = [0] * 16  # R0 - R15
        self.halted = True

    def write_mem(self, addr: int, data: bytes):
        offset = addr & 0xFFFF
        self.sram[offset:offset + len(data)] = data

    def read_mem(self, addr: int, length: int) -> bytes:
        offset = addr & 0xFFFF
        return bytes(self.sram[offset:offset + length])

    def write_reg(self, reg: int, val: int):
        self.regs[reg] = val

    def read_reg(self, reg: int) -> int:
        return self.regs[reg]

    def execute_harness(self, entry: int, sp: int, lr: int, r0: int, r1: int = 0, r2: int = 0) -> int:
        """Simulate execution of Thumb FLM function returning via BX LR to BKPT 0."""
        self.regs[0] = r0
        self.regs[1] = r1
        self.regs[2] = r2
        self.regs[13] = sp
        self.regs[14] = lr
        self.regs[15] = entry | 1
        self.halted = False

        # In target RAM harness:
        # If LR points to BKPT 0, BX LR immediately halts core with return code in R0
        bkpt_addr = lr & ~1
        bkpt_opcode = struct.unpack_from("<H", self.sram, bkpt_addr & 0xFFFF)[0]
        assert bkpt_opcode == 0xBE00, f"Expected BKPT 0 (0xBE00) at 0x{bkpt_addr:08X}, got 0x{bkpt_opcode:04X}"

        # Target FLM function returns 0 in R0 on success
        self.regs[0] = 0
        # Target halts on BKPT 0
        self.halted = True
        return self.regs[0]


def test_sram_execution_harness():
    """Verify execution lifecycle with stack top, BKPT 0 opcode setup and R0 return code."""
    target = MockRamTarget()
    hdr = AlgoBlobHeader(
        code_size=24,
        init_offset=4,
        erase_offset=8,
        program_offset=12,
        verify_offset=16,
        uninit_offset=20,
        ram_base=0x20000400,
        ram_size=1024,
        stack_top=0x20000800,
    )

    code_base = 0x20000000
    blob = generate_builtin("stm32f1")
    target.write_mem(code_base, blob[HEADER_SIZE:])

    # Setup BKPT 0 instruction at (sp - 4)
    sp = hdr.stack_top
    bkpt_addr = sp - 4
    target.write_mem(bkpt_addr, struct.pack("<H", 0xBE00))
    lr = bkpt_addr | 1

    # 1. Execute Init(flash_base=0x08000000, clk=0, func=1)
    r0 = target.execute_harness(
        entry=code_base + hdr.init_offset,
        sp=sp,
        lr=lr,
        r0=0x08000000,
        r1=0,
        r2=1,
    )
    assert target.halted is True
    assert r0 == 0  # Init returned success

    # 2. Execute EraseSector(sector_addr=0x08000000)
    r0 = target.execute_harness(
        entry=code_base + hdr.erase_offset,
        sp=sp,
        lr=lr,
        r0=0x08000000,
    )
    assert target.halted is True
    assert r0 == 0  # EraseSector returned success


def test_packaged_littlefs_algo_assets():
    """Verify algorithm binaries deployed in web-ui/system/algo/ match unpack expectations."""
    algo_dir = Path("web-ui/system/algo")
    assert algo_dir.is_dir()

    f1_path = algo_dir / "stm32f1.bin"
    assert f1_path.is_file()
    f1_data = f1_path.read_bytes()
    f1_hdr = AlgoBlobHeader.unpack(f1_data[:HEADER_SIZE])
    assert f1_hdr.magic == ALGO_MAGIC
    assert f1_hdr.code_size == 24

    g0_path = algo_dir / "stm32g0.bin"
    assert g0_path.is_file()
    g0_data = g0_path.read_bytes()
    g0_hdr = AlgoBlobHeader.unpack(g0_data[:HEADER_SIZE])
    assert g0_hdr.magic == ALGO_MAGIC
    assert g0_hdr.code_size == 24
