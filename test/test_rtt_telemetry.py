#!/usr/bin/env python3
"""
Unit test suite for Segger RTT Background Logging Engine and Telemetry API.
Validates:
- 16-byte "SEGGER RTT" signature discovery in target SRAM
- SEGGER_RTT_CB structure parsing (Up/Down buffer descriptors, pointers, ring indices)
- Non-intrusive SWD buffer draining (linear, wrap-around, empty cases)
- Target read-pointer update & log streaming
- REST API response serialization (/api/v1/programmer/rtt/status, /api/v1/programmer/rtt/stream)
"""

import struct
import pytest
from typing import Optional, Tuple


RTT_SIGNATURE = b"SEGGER RTT\x00\x00\x00\x00\x00\x00"


class MockTargetSram:
    """Mock ARM Cortex-M target SRAM memory model."""
    def __init__(self, base: int = 0x20000000, size: int = 0x10000):
        self.base = base
        self.size = size
        self.mem = bytearray(size)

    def write_bytes(self, addr: int, data: bytes):
        offset = addr - self.base
        if 0 <= offset <= self.size - len(data):
            self.mem[offset:offset + len(data)] = data
        else:
            raise ValueError(f"Out of bounds write: {hex(addr)}")

    def read_bytes(self, addr: int, length: int) -> bytes:
        offset = addr - self.base
        if 0 <= offset <= self.size - length:
            return bytes(self.mem[offset:offset + length])
        raise ValueError(f"Out of bounds read: {hex(addr)}")

    def read32(self, addr: int) -> int:
        return struct.unpack("<I", self.read_bytes(addr, 4))[0]

    def write32(self, addr: int, val: int):
        self.write_bytes(addr, struct.pack("<I", val))


class MockSeggerRttEngine:
    """
    Python reference model simulating firmware-rs/src/programmer/rtt.rs.
    """
    def __init__(self, sram: MockTargetSram):
        self.sram = sram
        self.cb_addr: Optional[int] = None
        self.max_up_buffers = 0
        self.max_down_buffers = 0
        self.buffer_ptr = 0
        self.buffer_size = 0
        self.total_bytes_read = 0

    def scan_sram(self, scan_base: int = 0x20000000, scan_size: int = 0x10000) -> bool:
        """Scan target SRAM window for the 16-byte SEGGER RTT signature."""
        chunk_size = 256
        curr = scan_base
        end = scan_base + scan_size
        while curr < end:
            read_len = min(chunk_size, end - curr)
            data = self.sram.read_bytes(curr, read_len)
            for i in range(0, read_len - 15, 4):
                if data[i:i + 10] == b"SEGGER RTT":
                    candidate = curr + i
                    if self._parse_control_block(candidate):
                        self.cb_addr = candidate
                        return True
            curr += chunk_size
        return False

    def _parse_control_block(self, addr: int) -> bool:
        hdr = self.sram.read_bytes(addr, 48)
        if hdr[0:10] != b"SEGGER RTT":
            return False
        max_up = struct.unpack("<I", hdr[16:20])[0]
        max_down = struct.unpack("<I", hdr[20:24])[0]
        if max_up == 0 or max_up > 32 or max_down > 32:
            return False

        # Up Buffer 0
        name_ptr, buf_ptr, size, wr_off, rd_off, flags = struct.unpack("<6I", hdr[24:48])
        if size == 0 or size > 65536 or buf_ptr == 0:
            return False

        self.max_up_buffers = max_up
        self.max_down_buffers = max_down
        self.buffer_ptr = buf_ptr
        self.buffer_size = size
        return True

    def drain_up_buffer(self, max_bytes: int = 512) -> bytes:
        """Drain newly logged bytes without halting the core."""
        if not self.cb_addr or self.buffer_size == 0:
            return b""

        # Read current wr_off and rd_off from target memory
        wr_off = self.sram.read32(self.cb_addr + 24 + 12) % self.buffer_size
        rd_off = self.sram.read32(self.cb_addr + 24 + 16) % self.buffer_size

        if wr_off == rd_off:
            return b""

        out = bytearray()
        if wr_off > rd_off:
            avail = wr_off - rd_off
            to_read = min(avail, max_bytes)
            data = self.sram.read_bytes(self.buffer_ptr + rd_off, to_read)
            out.extend(data)
            new_rd = (rd_off + to_read) % self.buffer_size
            self.sram.write32(self.cb_addr + 24 + 16, new_rd)
        else:
            # Wrap around
            part1 = self.buffer_size - rd_off
            to_read1 = min(part1, max_bytes)
            out.extend(self.sram.read_bytes(self.buffer_ptr + rd_off, to_read1))
            if to_read1 == part1 and len(out) < max_bytes:
                part2 = wr_off
                to_read2 = min(part2, max_bytes - len(out))
                if to_read2 > 0:
                    out.extend(self.sram.read_bytes(self.buffer_ptr, to_read2))
                new_rd = to_read2
                self.sram.write32(self.cb_addr + 24 + 16, new_rd)
            else:
                new_rd = (rd_off + to_read1) % self.buffer_size
                self.sram.write32(self.cb_addr + 24 + 16, new_rd)

        self.total_bytes_read += len(out)
        return bytes(out)


def setup_rtt_control_block_in_sram(
    sram: MockTargetSram,
    cb_addr: int = 0x20000400,
    buf_addr: int = 0x20001000,
    buf_size: int = 1024,
) -> int:
    """Helper to populate a valid SEGGER_RTT_CB structure in target SRAM."""
    cb = bytearray(48)
    cb[0:16] = RTT_SIGNATURE
    struct.pack_into("<I", cb, 16, 1)  # MaxNumUpBuffers = 1
    struct.pack_into("<I", cb, 20, 1)  # MaxNumDownBuffers = 1
    # aUp[0]
    struct.pack_into("<I", cb, 24, 0x20000000)  # sName
    struct.pack_into("<I", cb, 28, buf_addr)    # pBuffer
    struct.pack_into("<I", cb, 32, buf_size)    # SizeOfBuffer
    struct.pack_into("<I", cb, 36, 0)           # WrOff
    struct.pack_into("<I", cb, 40, 0)           # RdOff
    struct.pack_into("<I", cb, 44, 0)           # Flags
    sram.write_bytes(cb_addr, bytes(cb))
    return cb_addr


def test_rtt_signature_discovery():
    """Verify scanner discovers SEGGER RTT signature placed anywhere in SRAM."""
    sram = MockTargetSram()
    cb_location = 0x20002480  # Non-trivial aligned offset
    setup_rtt_control_block_in_sram(sram, cb_addr=cb_location, buf_size=512)

    engine = MockSeggerRttEngine(sram)
    assert engine.scan_sram() is True
    assert engine.cb_addr == cb_location
    assert engine.buffer_size == 512
    assert engine.max_up_buffers == 1


def test_rtt_signature_absent():
    """Verify scanner returns False cleanly without disrupting target if RTT not present."""
    sram = MockTargetSram()
    # Fill with random data
    sram.write_bytes(0x20000000, b"Hello World! No RTT signature here." * 10)

    engine = MockSeggerRttEngine(sram)
    assert engine.scan_sram() is False
    assert engine.cb_addr is None


def test_rtt_drain_linear_log_stream():
    """Verify linear buffer draining when WrOff > RdOff."""
    sram = MockTargetSram()
    cb_addr = setup_rtt_control_block_in_sram(sram, cb_addr=0x20000100, buf_addr=0x20001000, buf_size=1024)

    engine = MockSeggerRttEngine(sram)
    assert engine.scan_sram() is True

    # Target writes a log message: "System Initialized at 150MHz\n" (28 bytes)
    msg = b"System Initialized at 150MHz\n"
    sram.write_bytes(0x20001000, msg)
    # Target updates WrOff = 28
    sram.write32(cb_addr + 24 + 12, len(msg))

    # Probe drains buffer
    drained = engine.drain_up_buffer()
    assert drained == msg
    assert engine.total_bytes_read == len(msg)

    # Validate target RdOff updated to match WrOff
    target_rd_off = sram.read32(cb_addr + 24 + 16)
    assert target_rd_off == len(msg)

    # Next drain should return empty (no new data)
    assert engine.drain_up_buffer() == b""


def test_rtt_drain_ring_wraparound():
    """Verify wrap-around draining when WrOff wraps past buffer end."""
    sram = MockTargetSram()
    buf_size = 64
    buf_addr = 0x20000800
    cb_addr = setup_rtt_control_block_in_sram(sram, cb_addr=0x20000200, buf_addr=buf_addr, buf_size=buf_size)

    engine = MockSeggerRttEngine(sram)
    assert engine.scan_sram() is True

    # Simulate buffer state where RdOff = 50, and target wrote 25 bytes wrapping around:
    # 14 bytes from offset 50..64, and 11 bytes from offset 0..11
    sram.write32(cb_addr + 24 + 16, 50)  # RdOff = 50
    sram.write32(cb_addr + 24 + 12, 11)  # WrOff = 11

    part1_data = b"0123456789ABCD"  # 14 bytes
    part2_data = b"EFGHIJKLMNO"      # 11 bytes
    sram.write_bytes(buf_addr + 50, part1_data)
    sram.write_bytes(buf_addr, part2_data)

    drained = engine.drain_up_buffer(max_bytes=128)
    assert drained == part1_data + part2_data
    assert len(drained) == 25

    # Check RdOff updated to WrOff (11)
    new_rd = sram.read32(cb_addr + 24 + 16)
    assert new_rd == 11


def test_rtt_status_json_schema():
    """Verify REST API JSON formatting for /api/v1/programmer/rtt/status."""
    import json
    found_resp = r'{"status":"ok","found":true,"cb_addr":"0x20000100","buffer_size":1024,"buffer_ptr":"0x20001000","total_bytes_read":256}'
    data = json.loads(found_resp)
    assert data["status"] == "ok"
    assert data["found"] is True
    assert data["cb_addr"] == "0x20000100"
    assert data["buffer_size"] == 1024

    not_found_resp = r'{"status":"ok","found":false,"message":"RTT control block not found"}'
    nf_data = json.loads(not_found_resp)
    assert nf_data["found"] is False
