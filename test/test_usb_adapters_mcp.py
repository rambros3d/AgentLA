#!/usr/bin/env python3
"""
USB Adapters MCP tools test suite.

Validates:
- tools/list advertises usb_adapters_status and i3c_status.
- schema correctness and dispatch in firmware-rs/src/mcp.rs.
- tool count consistency in tools.rs, webui/mcp.json, and rest.rs.
"""

import json
import re
from pathlib import Path
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent


@pytest.fixture(scope="module")
def tools_rs():
    return (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")


@pytest.fixture(scope="module")
def firmware_mcp_rs():
    return (REPO_ROOT / "firmware-rs" / "src" / "mcp.rs").read_text(encoding="utf-8")


def test_usb_adapters_tools_in_catalog(tools_rs):
    for tool in ["usb_adapters_status", "i3c_status"]:
        const_name = "TOOL_" + tool.upper()
        assert f'"{tool}"' in tools_rs, f"{tool} missing from tools.rs"
        assert re.search(r"pub const " + const_name + r"\b", tools_rs), f"{const_name} const missing"
        assert re.search(r"^\s+" + const_name + r",\s*$", tools_rs, re.M), f"{const_name} not in ALL_TOOLS"
    assert "pub static ALL_TOOLS: [Tool; 78]" in tools_rs


def test_usb_adapters_dispatch_arms(firmware_mcp_rs):
    assert '"usb_adapters_status" =>' in firmware_mcp_rs
    assert '"i3c_status" =>' in firmware_mcp_rs
    assert '"unified_mega_composite"' in firmware_mcp_rs
    assert '"agentla-binary-bulk"' in firmware_mcp_rs
    assert '"i2c-tiny-usb"' in firmware_mcp_rs
    assert '"ftdi-mpsse"' in firmware_mcp_rs


def test_manifest_consistency():
    manifest_path = REPO_ROOT / "webui" / "mcp.json"
    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)
    assert data["capabilities"]["tools"]["count"] == 78


def test_rest_health_tool_count():
    rest_rs = (REPO_ROOT / "firmware-rs" / "src" / "rest.rs").read_text(encoding="utf-8")
    assert '"tools":78' in rest_rs
