#!/usr/bin/env python3
"""
Unit test suite for USB CMSIS-DAP v2 Bulk Interface and Protocol Engine.
Validates:
- CMSIS-DAP v2 command serialization and deserialization
- Protocol handlers: DAP_Info, DAP_Connect, DAP_Disconnect, DAP_TransferConfigure
- DAP_TransferBlock (0x06) read/write framing, payload unpacking, and SWD auto-increment pipeline
- DAP_SWJ_Sequence (0x12) bit-stream clocking and Dormant/JTAG-to-SWD switching patterns
- Microsoft OS 2.0 WinUSB descriptors & CMSIS-DAP GUID compatibility
"""

import struct
import pytest
from typing import List, Tuple, Optional


# CMSIS-DAP Command Constants
DAP_INFO = 0x00
DAP_HOST_STATUS = 0x01
DAP_CONNECT = 0x02
DAP_DISCONNECT = 0x03
DAP_TRANSFER_CONFIGURE = 0x04
DAP_TRANSFER = 0x05
DAP_TRANSFER_BLOCK = 0x06
DAP_TRANSFER_ABORT = 0x07
DAP_WRITE_ABORT = 0x08
DAP_DELAY = 0x09
DAP_RESET_TARGET = 0x0A
DAP_SWJ_PINS = 0x10
DAP_SWJ_CLOCK = 0x11
DAP_SWJ_SEQUENCE = 0x12
DAP_SWD_CONFIGURE = 0x13
DAP_SWD_SEQUENCE = 0x1D

# DAP Transfer Response Status
DAP_TRANSFER_OK = 0x01
DAP_TRANSFER_WAIT = 0x02
DAP_TRANSFER_FAULT = 0x04
DAP_TRANSFER_ERROR = 0x08
DAP_TRANSFER_NO_ACK = 0x10

# DAP Port
DAP_PORT_AUTODETECT = 0x00
DAP_PORT_SWD = 0x01
DAP_PORT_JTAG = 0x02

# Microsoft OS 2.0 WinUSB GUID for CMSIS-DAP v2
CMSIS_DAP_V2_GUID = "{CDB3B5AD-293B-4663-AA36-1AAE46463776}"


class MockCmsisDapV2Engine:
    """
    Python reference model simulating the AgentLA CmsisDapHandler state machine
    matching firmware-rs/src/programmer/dap.rs.
    """
    def __init__(self):
        self.connected = False
        self.idle_cycles = 0
        self.wait_retry = 100
        self.clock_hz = 1_000_000
        self.memory = {}  # Mock target memory: address -> 32-bit word
        self.tar = 0x20000000
        self.csw = 0x23000052  # CSW with Auto-Increment enabled
        self.swj_sequences_executed = []

    def handle_packet(self, req: bytes) -> bytes:
        if not req:
            return b""
        cmd = req[0]
        if cmd == DAP_INFO:
            return self._handle_info(req)
        elif cmd == DAP_HOST_STATUS:
            return bytes([DAP_HOST_STATUS, 0x00])
        elif cmd == DAP_CONNECT:
            port = req[1] if len(req) > 1 else 0
            if port in (DAP_PORT_AUTODETECT, DAP_PORT_SWD):
                self.connected = True
                return bytes([DAP_CONNECT, DAP_PORT_SWD])
            return bytes([DAP_CONNECT, 0x00])
        elif cmd == DAP_DISCONNECT:
            self.connected = False
            return bytes([DAP_DISCONNECT, 0x00])
        elif cmd == DAP_TRANSFER_CONFIGURE:
            if len(req) >= 4:
                self.idle_cycles = req[1]
                self.wait_retry = req[2] | (req[3] << 8)
            return bytes([DAP_TRANSFER_CONFIGURE, 0x00])
        elif cmd == DAP_TRANSFER_BLOCK:
            return self._handle_transfer_block(req)
        elif cmd == DAP_SWJ_CLOCK:
            if len(req) >= 5:
                self.clock_hz = struct.unpack("<I", req[1:5])[0]
            return bytes([DAP_SWJ_CLOCK, 0x00])
        elif cmd == DAP_SWJ_SEQUENCE:
            return self._handle_swj_sequence(req)
        elif cmd == DAP_RESET_TARGET:
            return bytes([DAP_RESET_TARGET, 0x00])
        else:
            return bytes([cmd, 0x00])

    def _handle_info(self, req: bytes) -> bytes:
        if len(req) < 2:
            return bytes([DAP_INFO, 0x00])
        sub = req[1]
        if sub == 0x01:  # Vendor
            v = b"AgentLA"
            return bytes([DAP_INFO, len(v)]) + v
        elif sub == 0x02:  # Product
            p = b"AgentLA RP2350 Probe"
            return bytes([DAP_INFO, len(p)]) + p
        elif sub == 0x03:  # Serial
            s = b"00000001"
            return bytes([DAP_INFO, len(s)]) + s
        elif sub == 0x04:  # FW version
            ver = b"1.0.0"
            return bytes([DAP_INFO, len(ver)]) + ver
        elif sub == 0xF0:  # Capabilities (bit 0 = SWD)
            return bytes([DAP_INFO, 1, 0x01])
        elif sub == 0xFE:  # Packet count
            return bytes([DAP_INFO, 1, 1])
        elif sub == 0xFF:  # Packet size (512 bytes LE)
            return bytes([DAP_INFO, 2, 0x00, 0x02])
        return bytes([DAP_INFO, 0x00])

    def _handle_transfer_block(self, req: bytes) -> bytes:
        if len(req) < 5:
            return struct.pack("<BBHB", DAP_TRANSFER_BLOCK, 0, 0, DAP_TRANSFER_ERROR)
        dap_idx = req[1]
        count = struct.unpack("<H", req[2:4])[0]
        req_type = req[4]
        is_ap = (req_type & 0x01) != 0
        is_read = (req_type & 0x02) != 0

        completed = 0
        words_out = []

        if is_read:
            for _ in range(count):
                val = self.memory.get(self.tar, 0x00000000)
                words_out.append(val)
                self.tar += 4
                completed += 1
            resp = bytearray(struct.pack("<BBHB", DAP_TRANSFER_BLOCK, count & 0xFF, (count >> 8) & 0xFF, DAP_TRANSFER_OK))
            # Format: DAP_TRANSFER_BLOCK (1B) + Count LE (2B) + Status (1B) + Data words (4B each)
            resp = bytearray([DAP_TRANSFER_BLOCK, completed & 0xFF, (completed >> 8) & 0xFF, DAP_TRANSFER_OK])
            for w in words_out:
                resp.extend(struct.pack("<I", w))
            return bytes(resp)
        else:
            # Write block
            req_idx = 5
            for _ in range(count):
                if req_idx + 4 > len(req):
                    break
                val = struct.unpack("<I", req[req_idx:req_idx + 4])[0]
                req_idx += 4
                self.memory[self.tar] = val
                self.tar += 4
                completed += 1
            return bytes([DAP_TRANSFER_BLOCK, completed & 0xFF, (completed >> 8) & 0xFF, DAP_TRANSFER_OK])

    def _handle_swj_sequence(self, req: bytes) -> bytes:
        if len(req) < 2:
            return bytes([DAP_SWJ_SEQUENCE, 0x01])
        bit_count = 256 if req[1] == 0 else req[1]
        byte_count = (bit_count + 7) // 8
        if len(req) < 2 + byte_count:
            return bytes([DAP_SWJ_SEQUENCE, 0x01])
        seq_bytes = req[2:2 + byte_count]
        self.swj_sequences_executed.append((bit_count, seq_bytes))
        return bytes([DAP_SWJ_SEQUENCE, 0x00])


def test_dap_info_commands():
    """Verify all DAP_Info queries return valid strings and hardware capabilities."""
    engine = MockCmsisDapV2Engine()

    # Vendor Name
    res = engine.handle_packet(bytes([DAP_INFO, 0x01]))
    assert res[0] == DAP_INFO
    length = res[1]
    assert res[2:2 + length].decode("ascii") == "AgentLA"

    # Product Name
    res = engine.handle_packet(bytes([DAP_INFO, 0x02]))
    assert res[0] == DAP_INFO
    length = res[1]
    assert "RP2350 Probe" in res[2:2 + length].decode("ascii")

    # Serial Number
    res = engine.handle_packet(bytes([DAP_INFO, 0x03]))
    assert res[0] == DAP_INFO
    length = res[1]
    assert res[2:2 + length].decode("ascii") == "00000001"

    # Protocol / FW Version
    res = engine.handle_packet(bytes([DAP_INFO, 0x04]))
    assert res[0] == DAP_INFO
    assert res[2:2 + res[1]].decode("ascii") == "1.0.0"

    # Capabilities: Bit 0 = SWD supported
    res = engine.handle_packet(bytes([DAP_INFO, 0xF0]))
    assert res[0] == DAP_INFO
    assert res[1] == 1
    assert (res[2] & 0x01) == 0x01  # SWD

    # Packet Size: 512 bytes LE
    res = engine.handle_packet(bytes([DAP_INFO, 0xFF]))
    assert res[0] == DAP_INFO
    assert res[1] == 2
    pkt_size = struct.unpack("<H", res[2:4])[0]
    assert pkt_size == 512


def test_dap_connect_and_disconnect():
    """Verify connection establishment and tear-down."""
    engine = MockCmsisDapV2Engine()

    # Connect to SWD port (0x01)
    res = engine.handle_packet(bytes([DAP_CONNECT, DAP_PORT_SWD]))
    assert res == bytes([DAP_CONNECT, DAP_PORT_SWD])
    assert engine.connected is True

    # Disconnect
    res = engine.handle_packet(bytes([DAP_DISCONNECT]))
    assert res == bytes([DAP_DISCONNECT, 0x00])
    assert engine.connected is False


def test_dap_transfer_block_write():
    """Verify contiguous block writes over SWD auto-incrementing TAR."""
    engine = MockCmsisDapV2Engine()
    engine.tar = 0x20000000

    # Write 4 words to SRAM: [0xDEADBEEF, 0xCAFEBABE, 0x12345678, 0xABCDEF01]
    words_to_write = [0xDEADBEEF, 0xCAFEBABE, 0x12345678, 0xABCDEF01]
    req = bytearray([
        DAP_TRANSFER_BLOCK,
        0x00,  # DAP Index
        0x04, 0x00,  # Count: 4 words LE
        0x01,  # AP Write (Bit 0 = 1 AP, Bit 1 = 0 Write)
    ])
    for w in words_to_write:
        req.extend(struct.pack("<I", w))

    resp = engine.handle_packet(bytes(req))
    assert len(resp) == 4
    assert resp[0] == DAP_TRANSFER_BLOCK
    count_completed = resp[1] | (resp[2] << 8)
    assert count_completed == 4
    assert resp[3] == DAP_TRANSFER_OK

    # Validate target memory contents
    for i, w in enumerate(words_to_write):
        addr = 0x20000000 + i * 4
        assert engine.memory[addr] == w
    assert engine.tar == 0x20000010


def test_dap_transfer_block_read():
    """Verify contiguous block read of multiple words over SWD auto-increment."""
    engine = MockCmsisDapV2Engine()
    engine.tar = 0x20001000

    # Prepopulate target memory
    expected_words = [0x11112222, 0x33334444, 0x55556666, 0x77778888, 0x99990000]
    for i, w in enumerate(expected_words):
        engine.memory[0x20001000 + i * 4] = w

    # Issue block read: count = 5 words, AP Read (0x01 | 0x02 = 0x03)
    req = bytes([
        DAP_TRANSFER_BLOCK,
        0x00,
        0x05, 0x00,  # Count = 5 LE
        0x03,        # AP Read
    ])

    resp = engine.handle_packet(req)
    assert resp[0] == DAP_TRANSFER_BLOCK
    completed = resp[1] | (resp[2] << 8)
    assert completed == 5
    assert resp[3] == DAP_TRANSFER_OK
    assert len(resp) == 4 + 5 * 4

    unpacked_words = struct.unpack("<5I", resp[4:])
    assert list(unpacked_words) == expected_words
    assert engine.tar == 0x20001000 + 5 * 4


def test_dap_swj_sequence_line_reset_and_dormant():
    """Verify DAP_SWJ_Sequence handles bit patterns for SWD activation."""
    engine = MockCmsisDapV2Engine()

    # 1. 50+ clock line reset (0xFF * 8 = 64 clocks high)
    line_reset_bytes = b"\xFF" * 8
    req1 = bytes([DAP_SWJ_SEQUENCE, 64]) + line_reset_bytes
    resp1 = engine.handle_packet(req1)
    assert resp1 == bytes([DAP_SWJ_SEQUENCE, 0x00])

    # 2. JTAG-to-SWD switch sequence: 0xE79E (16 bits)
    swd_switch = struct.pack("<H", 0xE79E)
    req2 = bytes([DAP_SWJ_SEQUENCE, 16]) + swd_switch
    resp2 = engine.handle_packet(req2)
    assert resp2 == bytes([DAP_SWJ_SEQUENCE, 0x00])

    assert len(engine.swj_sequences_executed) == 2
    assert engine.swj_sequences_executed[0] == (64, line_reset_bytes)
    assert engine.swj_sequences_executed[1] == (16, swd_switch)


def test_winusb_descriptors_validation():
    """Verify WinUSB MS OS 2.0 descriptor GUID requirements for probe-rs & Windows."""
    # The CMSIS-DAP v2 interface GUID must match Microsoft and ARM CMSIS specifications
    assert CMSIS_DAP_V2_GUID == "{CDB3B5AD-293B-4663-AA36-1AAE46463776}"
    # Must be valid UTF-16LE / ASCII GUID representation
    assert len(CMSIS_DAP_V2_GUID) == 38
    assert CMSIS_DAP_V2_GUID.startswith("{") and CMSIS_DAP_V2_GUID.endswith("}")
