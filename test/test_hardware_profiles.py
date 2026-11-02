#!/usr/bin/env python3
"""
Unit and integration tests for AgentLA Hardware-Resident Profiles and Safe State.
Verifies memory map, CRC32 slot headers, Safe State baseline, and profile operations.
"""

import struct
import zlib
import json
import pytest

PROFILE_TOTAL_SLOTS = 32
PROFILE_SLOT_SIZE = 2048
PROFILE_FLASH_OFFSET = 1536 * 1024  # 1.5MB
PROFILE_MAGIC = 0x41474C41          # 'AGLA'
PROFILE_NAME_MAX_LEN = 32
PROFILE_MAX_PAYLOAD_LEN = PROFILE_SLOT_SIZE - 48  # 2000 bytes

PROFILE_FLAG_IN_USE = (1 << 0)
PROFILE_FLAG_BOOT_DEFAULT = (1 << 1)
PROFILE_FLAG_READ_ONLY = (1 << 2)

HEADER_FORMAT = "<IHH32sII"  # magic (4), version (2), flags (2), name (32), crc (4), len (4)
HEADER_SIZE = struct.calcsize(HEADER_FORMAT)

class MockProfileSlot:
    def __init__(self, slot_id, name="Safe State (Hi-Z)", flags=0, payload="{}"):
        self.slot_id = slot_id
        self.magic = PROFILE_MAGIC
        self.version = 1
        self.flags = flags
        self.name = name.encode('utf-8')[:PROFILE_NAME_MAX_LEN-1]
        self.payload = payload.encode('utf-8')
        self.payload_len = len(self.payload)
        self.payload_crc32 = zlib.crc32(self.payload) & 0xFFFFFFFF

    def pack(self):
        padded_name = self.name.ljust(PROFILE_NAME_MAX_LEN, b'\x00')
        header = struct.pack(
            HEADER_FORMAT,
            self.magic,
            self.version,
            self.flags,
            padded_name,
            self.payload_crc32,
            self.payload_len
        )
        body = self.payload.ljust(PROFILE_MAX_PAYLOAD_LEN, b'\x00')
        data = header + body
        assert len(data) == PROFILE_SLOT_SIZE, f"Packed size is {len(data)}"
        return data

    @classmethod
    def unpack(cls, slot_id, data):
        assert len(data) == PROFILE_SLOT_SIZE
        magic, version, flags, name_bytes, crc, plen = struct.unpack_from(HEADER_FORMAT, data)
        assert magic == PROFILE_MAGIC, "Invalid magic"
        name = name_bytes.split(b'\x00')[0].decode('utf-8', errors='replace')
        payload = data[HEADER_SIZE:HEADER_SIZE + plen]
        calc_crc = zlib.crc32(payload) & 0xFFFFFFFF
        assert calc_crc == crc, f"CRC mismatch: expected {hex(crc)}, calculated {hex(calc_crc)}"
        return cls(slot_id, name, flags, payload.decode('utf-8'))


def test_profile_memory_geometry():
    """Verify flash sector alignment and capacity."""
    total_bytes = PROFILE_TOTAL_SLOTS * PROFILE_SLOT_SIZE
    assert total_bytes == 65536, "32 slots * 2KB must equal exactly 64KB"
    assert PROFILE_FLASH_OFFSET % 4096 == 0, "Profile block must be 4KB sector aligned"
    assert PROFILE_SLOT_SIZE == 2048, "Slot size must be 2048 bytes"


def test_slot_0_safe_state_contract():
    """Slot 0 must represent the read-only Safe State (All Hi-Z, 0V)."""
    slot0_payload = json.dumps({
        "name": "Safe State (Hi-Z)",
        "safe": True,
        "power": {"vtarget_enabled": False, "voltage_v": 0.0},
        "pins": [],
        "instruments": {}
    })
    slot0 = MockProfileSlot(
        slot_id=0,
        name="Safe State (Hi-Z)",
        flags=PROFILE_FLAG_IN_USE | PROFILE_FLAG_READ_ONLY,
        payload=slot0_payload
    )

    packed = slot0.pack()
    assert len(packed) == 2048
    unpacked = MockProfileSlot.unpack(0, packed)

    assert unpacked.magic == PROFILE_MAGIC
    assert (unpacked.flags & PROFILE_FLAG_READ_ONLY) != 0, "Slot 0 must be marked Read-Only"
    assert (unpacked.flags & PROFILE_FLAG_IN_USE) != 0

    parsed = json.loads(unpacked.payload.decode('utf-8'))
    assert parsed["safe"] is True
    assert parsed["power"]["vtarget_enabled"] is False
    assert parsed["power"]["voltage_v"] == 0.0


def test_slot_serialization_and_crc_validation():
    """Test saving and verifying a custom workbench profile."""
    custom_profile = {
        "name": "Automotive CAN-FD Sniffer",
        "board": "bakuprobe",
        "power": {"vtarget_enabled": True, "voltage_v": 3.3, "max_current_ma": 80},
        "pins": [
            {"pin": 10, "role": "CAN_TX", "term_120": True},
            {"pin": 11, "role": "CAN_RX"}
        ],
        "instruments": {
            "can_fd": {"nominal_baud": 500000, "data_baud": 2000000}
        }
    }
    payload_str = json.dumps(custom_profile)
    slot1 = MockProfileSlot(
        slot_id=1,
        name="Automotive CAN-FD Sniffer",
        flags=PROFILE_FLAG_IN_USE | PROFILE_FLAG_BOOT_DEFAULT,
        payload=payload_str
    )

    packed = slot1.pack()
    unpacked = MockProfileSlot.unpack(1, packed)
    assert unpacked.name.decode('utf-8') == "Automotive CAN-FD Sniffer"
    assert (unpacked.flags & PROFILE_FLAG_BOOT_DEFAULT) != 0

    # Tamper with payload byte and ensure CRC check catches corruption
    tampered = bytearray(packed)
    tampered[HEADER_SIZE + 5] ^= 0x55
    with pytest.raises(AssertionError, match="CRC mismatch"):
        MockProfileSlot.unpack(1, bytes(tampered))


def test_cross_board_compatibility_logic():
    """Simulate loading a BakuProbe 48-GPIO profile onto a Pico 2 (30-GPIO)."""
    baku_profile = {
        "name": "Full Rig Bringup",
        "board": "bakuprobe",
        "pins": [
            {"pin": 4, "role": "SWD_SWCLK"},
            {"pin": 5, "role": "SWD_SWDIO"},
            {"pin": 10, "role": "CAN_TX"},     # Hardware present on BakuProbe, absent on Pico2
            {"pin": 40, "role": "SCOPE_CH1"}   # GP40 out of range on Pico 2 (max GP29)
        ]
    }

    # Pico 2 environment capabilities
    pico2_max_gpio = 29
    pico2_features = {"has_hardware_can": False}

    applied_pins = []
    skipped_pins = []

    for p in baku_profile["pins"]:
        pin_num = p["pin"]
        role = p["role"]
        if pin_num > pico2_max_gpio:
            skipped_pins.append((pin_num, f"Pin GP{pin_num} exceeds max GPIO (GP{pico2_max_gpio})"))
        elif "CAN" in role and not pico2_features["has_hardware_can"]:
            skipped_pins.append((pin_num, "CAN transceiver hardware not present"))
        else:
            applied_pins.append((pin_num, role))

    assert len(applied_pins) == 2, "GP4 and GP5 should be applied"
    assert len(skipped_pins) == 2, "GP10 and GP40 should be gracefully skipped"
    assert applied_pins[0] == (4, "SWD_SWCLK")
    assert applied_pins[1] == (5, "SWD_SWDIO")


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
