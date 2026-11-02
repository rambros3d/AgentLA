#!/usr/bin/env python3
"""
Test suite for AgentLA board manifests, asset pipeline, and C header generation.
Verifies pico2, bakuprobe, and any additional board bundles.
"""

import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT))
BOARDS_DIR = REPO_ROOT / "boards"

import pytest

from tools.gen_board_assets import (
    validate_board_manifest,
    generate_header,
    stage_web_assets,
    REQUIRED_ROOT_KEYS,
    REQUIRED_ASSET_KEYS,
    REQUIRED_MCU_KEYS
)


def test_board_directories_exist():
    assert BOARDS_DIR.exists() and BOARDS_DIR.is_dir()
    expected_boards = ["pico2", "bakuprobe", "demo"]
    for b in expected_boards:
        b_path = BOARDS_DIR / b
        assert b_path.exists(), f"Expected board directory {b} missing"
        assert (b_path / "board.json").exists(), f"board.json missing for {b}"


@pytest.mark.parametrize("board_name", ["pico2", "bakuprobe", "demo"])
def test_manifest_schema_and_assets(board_name):
    board_dir = BOARDS_DIR / board_name
    manifest_file = board_dir / "board.json"
    
    with open(manifest_file, "r", encoding="utf-8") as f:
        data = json.load(f)
    
    # 1. Validation function must pass without exceptions
    validate_board_manifest(data, board_dir)
    
    # 2. Check board_id consistency
    assert data["board_id"] == board_name
    
    # 3. Check assets
    assets = data["assets"]
    for akey in REQUIRED_ASSET_KEYS:
        asset_file = board_dir / assets[akey]
        assert asset_file.exists()
        assert asset_file.stat().st_size > 0
        content = asset_file.read_text(encoding="utf-8")
        assert "<svg" in content and "</svg>" in content

    # 4. Check pinout.svg has .board-pin classes
    pinout_svg = (board_dir / assets["pinout"]).read_text(encoding="utf-8")
    assert 'class="board-pin"' in pinout_svg or "class='board-pin'" in pinout_svg
    assert 'id="pin-gp' in pinout_svg


def test_pico2_manifest_specifics():
    board_dir = BOARDS_DIR / "pico2"
    with open(board_dir / "board.json", "r", encoding="utf-8") as f:
        data = json.load(f)

    assert data["mcu"]["chip"].startswith("RP2350")
    assert data["features"]["has_hardware_can"] is False
    assert data["features"]["has_hardware_rs485"] is False
    assert data["gpio_range"] == [0, 29]
    assert "fixed_programmer" in data["zones"]
    assert "flexible_pool" in data["zones"]
    assert "analog_scope" in data["zones"]


def test_bakuprobe_manifest_specifics():
    board_dir = BOARDS_DIR / "bakuprobe"
    with open(board_dir / "board.json", "r", encoding="utf-8") as f:
        data = json.load(f)

    assert data["mcu"]["chip"] == "RP2350B"
    assert data["features"]["has_hardware_can"] is True
    assert data["features"]["has_hardware_rs485"] is True
    assert data["features"]["target_current_sense"] is True
    assert "zone1_programmer" in data["zones"]
    assert "zone2_transceivers" in data["zones"]
    assert "zone3_bus_masters" in data["zones"]
    assert "zone4_logic_analyzer" in data["zones"]
    assert "zone5_analog_scope" in data["zones"]
    assert data["zones"]["zone5_analog_scope"]["gpio_range"] == [40, 47]


def test_demo_manifest_specifics():
    board_dir = BOARDS_DIR / "demo"
    with open(board_dir / "board.json", "r", encoding="utf-8") as f:
        data = json.load(f)

    assert data["board_id"] == "demo"
    assert "Demo" in data["name"]
    assert data["mcu"]["chip"] == "RP2350B"
    assert data["gpio_range"] == [0, 47]
    assert data["features"]["has_hardware_can"] is True
    assert data["features"]["has_hardware_rs485"] is True
    assert data["features"]["target_current_sense"] is True
    assert data["features"]["target_power_control"] is True
    assert data["features"]["qwiic_stemma_qt"] is True
    assert "zone1_programmer" in data["zones"]
    assert "zone2_transceivers" in data["zones"]
    assert "zone3_bus_masters" in data["zones"]
    assert "zone4_logic_analyzer" in data["zones"]
    assert "zone5_analog_scope" in data["zones"]
    assert data["zones"]["zone5_analog_scope"]["gpio_range"] == [40, 47]


def test_header_generation():
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        header_pico2 = tmp_path / "board_config_pico2.h"
        header_baku = tmp_path / "board_config_baku.h"

        with open(BOARDS_DIR / "pico2" / "board.json", "r", encoding="utf-8") as f:
            data_pico2 = json.load(f)
        with open(BOARDS_DIR / "bakuprobe" / "board.json", "r", encoding="utf-8") as f:
            data_baku = json.load(f)

        generate_header(data_pico2, header_pico2)
        generate_header(data_baku, header_baku)

        assert header_pico2.exists()
        content_pico2 = header_pico2.read_text(encoding="utf-8")
        assert '#define AGENTLA_BOARD_ID           "pico2"' in content_pico2
        assert '#define BOARD_HAS_CAN              0' in content_pico2
        assert '#define BOARD_HAS_RS485            0' in content_pico2
        assert '#define BOARD_TARGET_CURRENT_SENSE 0' in content_pico2
        assert '#define BOARD_NET_IP_STR           "10.10.10.10"' in content_pico2
        assert '#define BOARD_NET_DOMAIN           "agentla.local"' in content_pico2
        assert '#define BOARD_NET_IP_A             10' in content_pico2

        # Verify board custom network override
        custom_board = dict(data_pico2)
        custom_board["network"] = {"ip": "192.168.42.1", "domain": "custom.local"}
        header_custom = tmp_path / "board_config_custom.h"
        generate_header(custom_board, header_custom)
        content_custom = header_custom.read_text(encoding="utf-8")
        assert '#define BOARD_NET_IP_STR           "192.168.42.1"' in content_custom
        assert '#define BOARD_NET_DOMAIN           "custom.local"' in content_custom
        assert '#define BOARD_NET_IP_A             192' in content_custom
        assert '#define BOARD_NET_IP_C             42' in content_custom

        assert header_baku.exists()
        content_baku = header_baku.read_text(encoding="utf-8")
        assert '#define AGENTLA_BOARD_ID           "bakuprobe"' in content_baku
        assert '#define BOARD_HAS_CAN              1' in content_baku
        assert '#define BOARD_HAS_RS485            1' in content_baku
        assert '#define BOARD_TARGET_CURRENT_SENSE 1' in content_baku
        assert "BOARD_MCU_FLASH_BYTES" in content_baku


def test_asset_staging_pipeline():
    with tempfile.TemporaryDirectory() as tmpdir:
        tmp_path = Path(tmpdir)
        output_web_dir = tmp_path / "web_staged"
        base_html_dir = REPO_ROOT / "firmware" / "html"

        with open(BOARDS_DIR / "pico2" / "board.json", "r", encoding="utf-8") as f:
            data = json.load(f)

        stage_web_assets(data, BOARDS_DIR / "pico2", output_web_dir, base_html_dir=base_html_dir)

        # Minified board.json exists and is valid JSON
        dest_json = output_web_dir / "board.json"
        assert dest_json.exists()
        min_manifest = json.loads(dest_json.read_text(encoding="utf-8"))
        assert min_manifest["board_id"] == "pico2"

        # Staged SVG assets exist
        assert (output_web_dir / "logo.svg").exists()
        assert (output_web_dir / "favicon.svg").exists()
        assert (output_web_dir / "pinout.svg").exists()

        # Base HTML files staged
        assert (output_web_dir / "index.html").exists()
        assert (output_web_dir / "app.js").exists()
        assert (output_web_dir / "style.css").exists()
