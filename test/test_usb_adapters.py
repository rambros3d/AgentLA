#!/usr/bin/env python3
"""
Unit verification for AgentLA USB Adapter SDKs (I3C and SPI).
Tests binary framing, header parsing, opcode definitions, and status codes.
"""

import sys
import os
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
import struct
from sdk.agentla.i3c import (
    I3C_BULK_MAGIC,
    I3C_HEADER_SIZE,
    I3C_OP_SET_BUS_SPEED,
    I3C_OP_PRIVATE_WRITE,
    I3C_OP_PRIVATE_READ,
    I3C_OP_ENTDAA,
    I3C_STATUS_OK,
    I3C_STATUS_NAK_ARBHDR,
    AgentLaI3c,
    I3cException,
)
from sdk.agentla.spi import AgentLaSpi

def test_i3c_binary_framing():
    opcode = I3C_OP_PRIVATE_WRITE
    flags = 0x0A # EMIT_STOP | PUSH_PULL
    seq = 42
    addr = 0x08
    payload = b"\x11\x22\x33\x44"
    header = struct.pack("<BBBBHBB", I3C_BULK_MAGIC, opcode, flags, seq, len(payload), addr, 0)
    packet = header + payload

    assert len(packet) == I3C_HEADER_SIZE + len(payload)
    magic, op, fl, sq, plen, target_addr, st = struct.unpack("<BBBBHBB", packet[:I3C_HEADER_SIZE])
    assert magic == 0x3C
    assert op == I3C_OP_PRIVATE_WRITE
    assert sq == 42
    assert target_addr == 0x08
    assert plen == 4
    assert packet[I3C_HEADER_SIZE:] == payload
    print("test_i3c_binary_framing: PASS")

def test_i3c_exception_formatting():
    try:
        raise I3cException(I3C_STATUS_NAK_ARBHDR, "Target not present")
    except I3cException as e:
        assert "ERR_NAK_ARBITRATION_HEADER" in str(e)
        assert "Target not present" in str(e)
    print("test_i3c_exception_formatting: PASS")

def test_adapter_classes_instantiation():
    i3c = AgentLaI3c(vid=0x1209, pid=0x414D)
    assert i3c.vid == 0x1209
    assert i3c.pid == 0x414D

    spi = AgentLaSpi(vid=0x1209, pid=0x414D)
    assert spi.vid == 0x1209
    assert spi.pid == 0x414D
    print("test_adapter_classes_instantiation: PASS")

if __name__ == "__main__":
    test_i3c_binary_framing()
    test_i3c_exception_formatting()
    test_adapter_classes_instantiation()
    print("ALL TESTS PASSED")
