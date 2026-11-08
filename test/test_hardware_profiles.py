#!/usr/bin/env python3
"""
Unit and integration tests for AgentLA Declarative Hardware Profiles stored in USER_FS.
Verifies memory map alignment, declarative JSON schema, safe state baseline,
and live HTTP REST profile operations against 10.10.10.10 if available.
"""

import os
import json
import pytest
import urllib.request
import urllib.error

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PROFILES_DIR = os.path.join(REPO_ROOT, "firmware-rs", "assets", "profiles")
DUT_URL = os.environ.get("AGENTLA_URL", "http://10.10.10.10")

def is_dut_reachable():
    try:
        req = urllib.request.Request(f"{DUT_URL}/api/board/manifest", method="GET")
        with urllib.request.urlopen(req, timeout=1.0) as resp:
            return resp.status == 200
    except Exception:
        return False


# --- Unit Tests: Flash Partition Geometry ---

def test_flash_memory_geometry():
    """Verify 4MB RP2350 5-zone partition alignment without raw profile slice."""
    flash_size = 4096 * 1024       # 4 MB
    boot_size = 16 * 1024          # 16 KB (0x00000000..0x00004000)
    slot_a_size = 1200 * 1024      # 1200 KB (0x00004000..0x00130000)
    slot_b_size = 1216 * 1024      # 1216 KB (0x00130000..0x00260000 window)
    system_fs_size = 512 * 1024    # 512 KB (0x00260000..0x002E0000)
    user_fs_size = 1136 * 1024     # 1136 KB (0x002E0000..0x003FC000)
    trail_size = 16 * 1024         # 16 KB (0x003FC000..0x00400000)

    assert boot_size + slot_a_size + slot_b_size + system_fs_size + user_fs_size + trail_size == flash_size

    # Verify sector alignment
    assert 0x00000000 % 4096 == 0
    assert 0x00004000 % 4096 == 0
    assert 0x00130000 % 4096 == 0
    assert 0x00260000 % 4096 == 0
    assert 0x002E0000 % 4096 == 0
    assert 0x003FC000 % 4096 == 0

    # Ensure USER_FS starts immediately after SYSTEM_FS (no gap)
    system_fs_end = 0x00260000 + system_fs_size
    assert system_fs_end == 0x002E0000, "USER_FS must directly follow SYSTEM_FS"


# --- Unit Tests: Declarative JSON Profile Schema & Safe State Contract ---

def test_rom_template_profiles_exist():
    """Verify all 4 default template profiles exist in firmware-rs/assets/profiles/."""
    expected = ["safe_state.json", "logic_analyzer.json", "can_sniffer.json", "uart_passthrough.json"]
    for fname in expected:
        fpath = os.path.join(PROFILES_DIR, fname)
        assert os.path.exists(fpath), f"Missing embedded profile template: {fname}"


def test_safe_state_contract():
    """Safe State must be strictly read-only, 0V target, and no GPIO drivers."""
    safe_path = os.path.join(PROFILES_DIR, "safe_state.json")
    with open(safe_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    assert data["id"] == "safe_state"
    assert data["read_only"] is True
    assert data["is_boot"] is True
    assert data["power"]["v_target_enabled"] is False
    assert data["power"]["v_target_mv"] == 0
    assert len(data["pins"]) == 0


def test_declarative_profile_schemas():
    """Validate all template profiles against the declarative schema rules."""
    templates = ["safe_state.json", "logic_analyzer.json", "can_sniffer.json", "uart_passthrough.json"]
    for fname in templates:
        fpath = os.path.join(PROFILES_DIR, fname)
        with open(fpath, "r", encoding="utf-8") as f:
            p = json.load(f)

        assert "id" in p and len(p["id"]) <= 32
        assert "name" in p and len(p["name"]) <= 48
        assert "description" in p and len(p["description"]) <= 96
        assert isinstance(p.get("is_boot"), bool)
        assert isinstance(p.get("read_only"), bool)
        assert "power" in p
        assert isinstance(p["power"]["v_target_enabled"], bool)
        assert 0 <= p["power"]["v_target_mv"] <= 5000
        assert 0 <= p["power"]["overcurrent_limit_ma"] <= 2000
        assert isinstance(p.get("pins"), list)

        # Check pin bounds
        for pin_item in p["pins"]:
            assert 0 <= pin_item["pin"] <= 29, f"Pin GP{pin_item['pin']} exceeds RP2350 limits"
            assert "driver" in pin_item
            assert "label" in pin_item


def test_cross_board_compatibility_logic():
    """Simulate loading a BakuProbe 48-GPIO profile onto a Pico 2 (30-GPIO)."""
    baku_profile = {
        "id": "full_rig_bringup",
        "name": "Full Rig Bringup",
        "pins": [
            {"pin": 4, "driver": "swd", "label": "SWCLK"},
            {"pin": 5, "driver": "swd", "label": "SWDIO"},
            {"pin": 10, "driver": "can", "label": "CAN_TX"},
            {"pin": 40, "driver": "scope", "label": "CH1"}
        ]
    }

    pico2_max_gpio = 29
    pico2_features = {"can": False, "swd": True, "scope": False}

    applied_pins = []
    skipped_pins = []

    for p in baku_profile["pins"]:
        pin_num = p["pin"]
        driver = p["driver"]
        if pin_num > pico2_max_gpio:
            skipped_pins.append((pin_num, f"Pin GP{pin_num} exceeds max GPIO (GP{pico2_max_gpio})"))
        elif not pico2_features.get(driver, False):
            skipped_pins.append((pin_num, f"Driver {driver} not supported"))
        else:
            applied_pins.append((pin_num, driver))

    assert len(applied_pins) == 2, "GP4 and GP5 should be applied"
    assert len(skipped_pins) == 2, "GP10 and GP40 should be gracefully skipped"
    assert applied_pins[0] == (4, "swd")
    assert applied_pins[1] == (5, "swd")


# --- Live Integration Tests (executed if DUT is reachable on 10.10.10.10) ---

@pytest.mark.skipif(not is_dut_reachable(), reason="AgentLA hardware is not reachable on network")
class TestLiveHardwareProfiles:

    def test_get_profiles_list(self):
        req = urllib.request.Request(f"{DUT_URL}/api/v1/profiles", method="GET")
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert "profiles" in data
            assert isinstance(data["profiles"], list)
            assert len(data["profiles"]) >= 1
            ids = [p["id"] for p in data["profiles"]]
            assert "safe_state" in ids
            assert data["active_profile"] == "safe_state"

    def test_apply_safe_state(self):
        payload = json.dumps({"id": "safe_state"}).encode("utf-8")
        req = urllib.request.Request(
            f"{DUT_URL}/api/v1/profiles/apply",
            data=payload,
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200
            data = json.loads(resp.read().decode("utf-8"))
            assert data["ok"] is True
            assert data["id"] == "safe_state"

    def test_custom_profile_crud_cycle(self):
        test_id = "test_pytest"
        custom_doc = {
            "id": test_id,
            "name": "Pytest Temp Profile",
            "description": "Created during pytest integration run",
            "is_boot": False,
            "read_only": False,
            "power": {
                "v_target_enabled": False,
                "v_target_mv": 0,
                "overcurrent_limit_ma": 500
            },
            "pins": [
                {"pin": 12, "driver": "capture", "label": "TEST_D0"}
            ],
            "instruments": {}
        }

        # 1. Save profile
        req = urllib.request.Request(
            f"{DUT_URL}/api/v1/profiles/save",
            data=json.dumps(custom_doc).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200
            assert json.loads(resp.read().decode("utf-8"))["ok"] is True

        # 2. Verify in list
        req = urllib.request.Request(f"{DUT_URL}/api/v1/profiles", method="GET")
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            data = json.loads(resp.read().decode("utf-8"))
            ids = [p["id"] for p in data["profiles"]]
            assert test_id in ids

        # 3. Apply custom profile
        req = urllib.request.Request(
            f"{DUT_URL}/api/v1/profiles/apply",
            data=json.dumps({"id": test_id}).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200

        # 4. Revert to safe state
        req = urllib.request.Request(
            f"{DUT_URL}/api/v1/profiles/apply",
            data=json.dumps({"id": "safe_state"}).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200

        # 5. Delete profile
        req = urllib.request.Request(
            f"{DUT_URL}/api/v1/profiles/delete",
            data=json.dumps({"id": test_id}).encode("utf-8"),
            headers={"Content-Type": "application/json"},
            method="POST"
        )
        with urllib.request.urlopen(req, timeout=2.0) as resp:
            assert resp.status == 200
            assert json.loads(resp.read().decode("utf-8"))["ok"] is True


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
