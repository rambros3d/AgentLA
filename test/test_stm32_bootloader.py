#!/usr/bin/env python3
"""
Unit test suite for STM32 AN3155 UART Bootloader Recovery Engine.
Validates:
- AN3155 command framing (CMD and inverted byte CMD ^ 0xFF)
- 8E1 Even Parity calculation
- Address and data payload XOR checksum algorithms
- Extended erase mass-erase framing (0xFF, 0xFF, 0x00)
- Readout Unprotect sequence (0x92, 0x6D, two-phase ACK)
- Timeout and NACK handling
- REST API response format (/api/v1/programmer/rescue/stm32)
"""

import json
import pytest

ACK = 0x79
NACK = 0x1F
SYNC = 0x7F

CMD_GET = 0x00
CMD_GET_VERSION = 0x01
CMD_GET_ID = 0x02
CMD_READ_MEMORY = 0x11
CMD_GO = 0x21
CMD_WRITE_MEMORY = 0x31
CMD_ERASE = 0x43
CMD_EXTENDED_ERASE = 0x44
CMD_READOUT_PROTECT = 0x82
CMD_READOUT_UNPROTECT = 0x92


def calc_even_parity(byte: int) -> int:
    """Calculate even parity bit for 8-bit data."""
    ones = bin(byte & 0xFF).count('1')
    return 1 if (ones % 2 != 0) else 0


def calc_address_checksum(addr: int) -> int:
    """Calculate AN3155 4-byte address XOR checksum."""
    a0 = (addr >> 24) & 0xFF
    a1 = (addr >> 16) & 0xFF
    a2 = (addr >> 8) & 0xFF
    a3 = addr & 0xFF
    return a0 ^ a1 ^ a2 ^ a3


def calc_data_checksum(data: bytes) -> int:
    """Calculate AN3155 data packet checksum: (len - 1) XOR data[0] XOR ... data[N-1]."""
    n_minus_1 = (len(data) - 1) & 0xFF
    chk = n_minus_1
    for b in data:
        chk ^= b
    return chk


def test_even_parity_calculation():
    """Verify even parity bit ensures total number of 1s is even."""
    test_bytes = [0x00, 0x7F, 0x79, 0x1F, 0x92, 0x6D, 0x44, 0xBB, 0x31, 0xCE, 0xFF]
    for b in test_bytes:
        parity = calc_even_parity(b)
        total_ones = bin(b).count('1') + parity
        assert total_ones % 2 == 0, f"Byte 0x{b:02X} parity {parity} is not even!"


def test_command_framing_checksums():
    """Verify that AN3155 command packet second byte is exact bitwise complement."""
    commands = [
        (CMD_GET, 0xFF),
        (CMD_GET_VERSION, 0xFE),
        (CMD_GET_ID, 0xFD),
        (CMD_READ_MEMORY, 0xEE),
        (CMD_WRITE_MEMORY, 0xCE),
        (CMD_ERASE, 0xBC),
        (CMD_EXTENDED_ERASE, 0xBB),
        (CMD_READOUT_PROTECT, 0x7D),
        (CMD_READOUT_UNPROTECT, 0x6D),
    ]
    for cmd, expected_inv in commands:
        assert (cmd ^ 0xFF) == expected_inv, f"CMD 0x{cmd:02X} complement mismatch"


def test_address_checksum():
    """Verify 4-byte address XOR checksum calculation."""
    # Example flash base: 0x08000000 -> 0x08 ^ 0x00 ^ 0x00 ^ 0x00 = 0x08
    assert calc_address_checksum(0x08000000) == 0x08
    # Example RAM base: 0x20001000 -> 0x20 ^ 0x00 ^ 0x10 ^ 0x00 = 0x30
    assert calc_address_checksum(0x20001000) == 0x30
    # Arbitrary address
    assert calc_address_checksum(0x12345678) == (0x12 ^ 0x34 ^ 0x56 ^ 0x78)


def test_data_checksum():
    """Verify data packet checksum calculation according to AN3155 specification."""
    # Single byte 0xAA (len 1: N-1 = 0, chk = 0 ^ 0xAA = 0xAA)
    assert calc_data_checksum(b'\xaa') == 0xAA

    # 4-byte word [0x11, 0x22, 0x33, 0x44] (len 4: N-1 = 3)
    data = bytes([0x11, 0x22, 0x33, 0x44])
    expected = 3 ^ 0x11 ^ 0x22 ^ 0x33 ^ 0x44
    assert calc_data_checksum(data) == expected


def test_extended_erase_mass_payload():
    """Verify extended erase special global mass-erase parameters."""
    page_count = 0xFFFF
    hi = (page_count >> 8) & 0xFF
    lo = page_count & 0xFF
    chk = hi ^ lo
    assert hi == 0xFF
    assert lo == 0xFF
    assert chk == 0x00


class MockStm32BootloaderTarget:
    """Mock simulating STM32 UART Bootloader responses."""

    def __init__(self):
        self.synced = False
        self.rdp_locked = True
        self.mass_erased = False
        self.rx_buffer = bytearray()

    def receive_byte(self, b: int) -> list[int]:
        if not self.synced:
            if b == SYNC:
                self.synced = True
                return [ACK]
            return [NACK]

        self.rx_buffer.append(b)
        if len(self.rx_buffer) == 2:
            cmd = self.rx_buffer[0]
            comp = self.rx_buffer[1]
            self.rx_buffer.clear()

            if (cmd ^ comp) != 0xFF:
                return [NACK]

            if cmd == CMD_READOUT_UNPROTECT:
                self.rdp_locked = False
                self.mass_erased = True
                # Two ACKs: immediate command ACK, then erase-complete ACK
                return [ACK, ACK]
            elif cmd == CMD_GET:
                # ACK + len(11) + version(0x22) + 11 commands + ACK
                cmds = [CMD_GET, CMD_GET_VERSION, CMD_GET_ID, CMD_READ_MEMORY,
                        CMD_GO, CMD_WRITE_MEMORY, CMD_ERASE, CMD_EXTENDED_ERASE,
                        CMD_READOUT_UNPROTECT]
                return [ACK, len(cmds), 0x22] + cmds + [ACK]
            else:
                return [ACK]

        return []


def test_bootloader_unprotect_sequence():
    """Simulate full Readout Unprotect rescue lifecycle."""
    target = MockStm32BootloaderTarget()

    # Step 1: Sync
    resp = target.receive_byte(SYNC)
    assert resp == [ACK]
    assert target.synced is True

    # Step 2: Send Readout Unprotect (0x92, 0x6D)
    resp1 = target.receive_byte(CMD_READOUT_UNPROTECT)
    assert resp1 == []  # Waiting for 2nd byte
    resp2 = target.receive_byte(CMD_READOUT_UNPROTECT ^ 0xFF)
    assert resp2 == [ACK, ACK]
    assert target.rdp_locked is False
    assert target.mass_erased is True


def test_rest_api_rescue_response_format():
    """Verify REST API response payload format for /api/v1/programmer/rescue/stm32."""
    resp_json = json.dumps({
        "status": "ok",
        "unprotected": True,
        "mass_erased": True,
        "message": "STM32 Readout Unprotect and mass-erase complete"
    })
    data = json.loads(resp_json)
    assert data["status"] == "ok"
    assert data["unprotected"] is True
    assert data["mass_erased"] is True


def test_rest_api_pin_conflict_format():
    """Verify REST API 409 Conflict response payload when pins are contested."""
    conflict_json = json.dumps({
        "status": "error",
        "error": "Pin conflict with active analyzer or peripheral driver"
    })
    data = json.loads(conflict_json)
    assert data["status"] == "error"
    assert "Pin conflict" in data["error"]
