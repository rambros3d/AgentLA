#!/usr/bin/env python3
"""
Unit test suite for AgentLA MCP Firmware Vector Table & Layout Inspector (`mcp_inspect_firmware`).
Validates:
- Flash vector table parsing (Initial SP, Reset Vector, NMI, HardFault).
- Dynamic Stack Pointer boundary validation against LittleFS AlgoBlobHeader (ram_base, ram_size).
- ARM Cortex-M Thumb mode invariant check ((Reset_Vector & 1) == 1).
- Target Option Byte RDP (Readout Protection) level readout.
- Structured JSON output format and warning generation.
"""

import json
import struct
import pytest
from typing import Optional, NamedTuple


class VectorTable(NamedTuple):
    initial_sp: int
    reset_vector: int
    nmi_vector: int
    hardfault_vector: int


def parse_vector_table(data: bytes) -> Optional[VectorTable]:
    if len(data) < 16:
        return None
    sp, reset, nmi, hf = struct.unpack("<4I", data[:16])
    return VectorTable(sp, reset, nmi, hf)


def validate_stack_pointer(sp: int, ram_base: int, ram_size: int) -> bool:
    # SP must be word aligned and lie within physical target SRAM bounds
    return (sp & 3) == 0 and ram_base <= sp <= (ram_base + ram_size)


def validate_thumb_bit(reset_vector: int) -> bool:
    # Cortex-M mandates bit 0 must be 1
    return (reset_vector & 1) == 1


def inspect_firmware_layout(
    vector_bytes: bytes,
    ram_base: int = 0x20000000,
    ram_size: int = 20 * 1024,
    rdp_byte: int = 0xA5,
    flash_addr: int = 0x08000000,
) -> dict:
    vt = parse_vector_table(vector_bytes)
    if vt is None:
        return {"error": "Invalid vector table length"}

    sp_valid = validate_stack_pointer(vt.initial_sp, ram_base, ram_size)
    thumb_valid = validate_thumb_bit(vt.reset_vector)

    if rdp_byte in (0xA5, 0xAA):
        rdp_level = "Level 0 (unprotected)"
    elif rdp_byte == 0xCC:
        rdp_level = "Level 2 (locked)"
    else:
        rdp_level = "Level 1 (readout protected)"

    valid = sp_valid and thumb_valid
    warnings = []
    if not sp_valid:
        warnings.append(
            f"Initial SP 0x{vt.initial_sp:08X} out of SRAM bounds [0x{ram_base:08X} - 0x{ram_base + ram_size:08X}]"
        )
    if not thumb_valid:
        warnings.append(
            f"Reset Vector 0x{vt.reset_vector:08X} missing Thumb bit (bit 0 must be 1)"
        )

    warning_str = "; ".join(warnings) if warnings else None

    return {
        "flash_address": f"0x{flash_addr:08x}",
        "initial_sp": f"0x{vt.initial_sp:08x}",
        "reset_vector": f"0x{vt.reset_vector:08x}",
        "nmi_vector": f"0x{vt.nmi_vector:08x}",
        "hardfault_vector": f"0x{vt.hardfault_vector:08x}",
        "sram_base": f"0x{ram_base:08x}",
        "sram_size": ram_size,
        "sp_valid": sp_valid,
        "reset_vector_thumb_valid": thumb_valid,
        "rdp_level": rdp_level,
        "valid": valid,
        "warning": warning_str,
    }


def test_parse_valid_vector_table():
    # STM32F1 typical binary vector header
    raw = struct.pack("<4I", 0x20005000, 0x08000145, 0x08000109, 0x0800010D)
    vt = parse_vector_table(raw)
    assert vt is not None
    assert vt.initial_sp == 0x20005000
    assert vt.reset_vector == 0x08000145
    assert vt.nmi_vector == 0x08000109
    assert vt.hardfault_vector == 0x0800010D


def test_parse_truncated_vector_table():
    raw = b"\x00\x50\x00\x20"  # Only 4 bytes
    assert parse_vector_table(raw) is None


def test_stack_pointer_validation_within_bounds():
    # Target SRAM: 0x20000000 to 0x20005000 (20 KB)
    ram_base = 0x20000000
    ram_size = 20 * 1024  # 0x5000

    # Top of stack
    assert validate_stack_pointer(0x20005000, ram_base, ram_size) is True
    # Inside SRAM
    assert validate_stack_pointer(0x20002000, ram_base, ram_size) is True
    # Base of SRAM
    assert validate_stack_pointer(0x20000000, ram_base, ram_size) is True


def test_stack_pointer_validation_out_of_bounds():
    ram_base = 0x20000000
    ram_size = 20 * 1024  # 0x5000

    # Overflowing SRAM (e.g. linked for 64KB on 20KB part)
    assert validate_stack_pointer(0x20010000, ram_base, ram_size) is False
    # Flash address mistakenly used as SP
    assert validate_stack_pointer(0x08000000, ram_base, ram_size) is False
    # Unaligned SP
    assert validate_stack_pointer(0x20004FFF, ram_base, ram_size) is False


def test_thumb_bit_invariant():
    # Odd address (Thumb bit set)
    assert validate_thumb_bit(0x08000145) is True
    assert validate_thumb_bit(0x08000001) is True

    # Even address (Missing Thumb bit -> triggers immediate HardFault on ARM core)
    assert validate_thumb_bit(0x08000144) is False
    assert validate_thumb_bit(0x08000140) is False


def test_rdp_level_decoding():
    raw = struct.pack("<4I", 0x20005000, 0x08000145, 0x08000109, 0x0800010D)

    # Level 0 (unprotected)
    rep0 = inspect_firmware_layout(raw, rdp_byte=0xA5)
    assert rep0["rdp_level"] == "Level 0 (unprotected)"

    # Level 1 (protected)
    rep1 = inspect_firmware_layout(raw, rdp_byte=0x00)
    assert rep1["rdp_level"] == "Level 1 (readout protected)"

    # Level 2 (permanently locked)
    rep2 = inspect_firmware_layout(raw, rdp_byte=0xCC)
    assert rep2["rdp_level"] == "Level 2 (locked)"


def test_healthy_firmware_inspection():
    raw = struct.pack("<4I", 0x20005000, 0x08000145, 0x08000109, 0x0800010D)
    rep = inspect_firmware_layout(raw, ram_base=0x20000000, ram_size=20480)
    assert rep["valid"] is True
    assert rep["sp_valid"] is True
    assert rep["reset_vector_thumb_valid"] is True
    assert rep["warning"] is None


def test_invalid_sp_and_missing_thumb_warning_synthesis():
    # SP points to 0x30000000 (invalid), Reset Vector is 0x08000144 (even, missing Thumb bit)
    raw = struct.pack("<4I", 0x30000000, 0x08000144, 0x08000109, 0x0800010D)
    rep = inspect_firmware_layout(raw, ram_base=0x20000000, ram_size=20480)
    assert rep["valid"] is False
    assert rep["sp_valid"] is False
    assert rep["reset_vector_thumb_valid"] is False
    assert "Initial SP 0x30000000 out of SRAM bounds" in rep["warning"]
    assert "missing Thumb bit" in rep["warning"]


def test_mcp_json_serialization_roundtrip():
    raw = struct.pack("<4I", 0x20005000, 0x08000145, 0x08000109, 0x0800010D)
    rep = inspect_firmware_layout(raw)
    json_text = json.dumps(rep)
    envelope = {
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "content": [{"type": "text", "text": json_text}],
            "isError": False,
        },
    }
    dumped = json.dumps(envelope)
    loaded = json.loads(dumped)
    payload = json.loads(loaded["result"]["content"][0]["text"])
    assert payload["valid"] is True
    assert payload["sp_valid"] is True
    assert payload["initial_sp"] == "0x20005000"
