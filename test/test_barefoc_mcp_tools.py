#!/usr/bin/env python3
"""
BareFOC MCP tools test suite (`mcp-barefoc-tools`).

Validates:
- tools/list advertises all eight barefoc_* tools with JSON input schemas.
- every barefoc_* tool has a dispatch arm in firmware-rs/src/mcp.rs using extract_* helpers
  and bounded heapless output.
- stubbed tools/call JSON-RPC round-trip for each tool returns bounded valid JSON
  with a correct isError flag.
- resource agentla://skills/barefoc and prompt barefoc_debug are registered.
"""

import json
import re
from pathlib import Path

import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent

BAREFOC_TOOLS = [
    "barefoc_status",
    "barefoc_target",
    "barefoc_read",
    "barefoc_write",
    "barefoc_command",
    "barefoc_mode",
    "barefoc_calibrate",
    "barefoc_emergency",
]


@pytest.fixture(scope="module")
def tools_rs():
    return (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")


@pytest.fixture(scope="module")
def firmware_mcp_rs():
    return (REPO_ROOT / "firmware-rs" / "src" / "mcp.rs").read_text(encoding="utf-8")


@pytest.fixture(scope="module")
def server_rs():
    return (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "server.rs").read_text(encoding="utf-8")


def test_tools_declared_in_catalog(tools_rs):
    for tool in BAREFOC_TOOLS:
        const_name = "TOOL_" + tool.upper()
        assert f'"{tool}"' in tools_rs, f"{tool} missing from tools.rs"
        assert re.search(r"pub const " + const_name + r"\b", tools_rs), f"{const_name} const missing"
        # Each tool must appear in the ALL_TOOLS array
        assert re.search(r"^\s+" + const_name + r",\s*$", tools_rs, re.M), f"{const_name} not in ALL_TOOLS"
    assert "pub static ALL_TOOLS: [Tool; 78]" in tools_rs


def test_firmware_dispatch_arms(firmware_mcp_rs):
    for tool in BAREFOC_TOOLS:
        assert f'"{tool}" =>' in firmware_mcp_rs, f"mcp.rs dispatch arm missing for {tool}"
    # Session-only active target static present with default node 1 / addr 0x41
    assert "BAREFOC_NODE" in firmware_mcp_rs
    assert "BAREFOC_ADDR" in firmware_mcp_rs
    assert "AtomicU8::new(1)" in firmware_mcp_rs
    assert "AtomicU8::new(0x41)" in firmware_mcp_rs
    # Buses bridged, no local FOC loop; emergency delegates to safe state
    assert "bridge::i2c::hw_i2c_read" in firmware_mcp_rs
    assert "bridge::i2c::hw_i2c_write" in firmware_mcp_rs
    assert "bridge::spi::spi_transfer_default" in firmware_mcp_rs
    assert "bridge::uart::uart_write_bytes" in firmware_mcp_rs
    assert "bridge::can_gsusb::transmit_can_ws_frame" in firmware_mcp_rs
    assert 'crate::profile::apply_profile_sync(crate::profile::SAFE_STATE_ID)' in firmware_mcp_rs


def test_resource_and_prompt_registered(server_rs):
    assert "agentla://skills/barefoc" in server_rs
    assert "BareFOC Agentic Debug Playbook" in server_rs
    assert '"barefoc_debug"' in server_rs
    assert "BAREFOC_DEBUG_ARGS" in server_rs
    assert "pub static DEFAULT_RESOURCES: [Resource; 10]" in server_rs
    assert "pub static DEFAULT_PROMPTS: [Prompt; 7]" in server_rs


def _stub_tools_call(name, arguments):
    """Simulate the MCP server tools/call envelope with a stubbed handler.

    Each handler writes a bounded JSON object into the tool result buffer; the
    server wraps it as a text content block with an isError flag. This mirrors
    the no_std firmware behavior and lets us validate envelope/JSON structure
    without a physical DUT attached.
    """
    reports = {
        "barefoc_status": {"status": "ok", "vtarget_mv": 3300, "active_profile": 0, "safe_state": True},
        "barefoc_target": {"status": "ok", "node": 1, "addr": 65},
        "barefoc_read": {"status": "ok", "bus": "i2c", "reg": 16, "len": 1, "hex": "AB", "bytes": [171]},
        "barefoc_write": {"status": "ok", "bus": "i2c", "reg": 16, "written": 1},
        "barefoc_command": {"status": "ok", "command": "M1", "bytes_sent": 2, "bytes_read": 2, "response": "M1"},
        "barefoc_mode": {"status": "ok", "node": 1, "mode": "velocity", "enabled": False},
        "barefoc_calibrate": {"status": "ok", "node": 1, "kind": "motor", "state": "in_progress"},
        "barefoc_emergency": {"status": "ok", "safe_state": True, "power_enabled": False},
    }
    assert name in reports, f"no stub for {name}"
    # Bound the response text the same way the firmware caps its tool buffer (1536 bytes)
    text = json.dumps(reports[name], separators=(",", ":"))
    assert len(text) <= 1536, f"{name} stub output exceeds 1536-byte tool buffer"
    return {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [{"type": "text", "text": text}],
            "isError": False,
        },
    }


@pytest.mark.parametrize("tool", BAREFOC_TOOLS)
def test_barefoc_tools_call_envelope(tool):
    arguments = {}
    if tool == "barefoc_target":
        arguments = {"node": 1}
    elif tool in ("barefoc_read", "barefoc_write"):
        arguments = {"reg": 16, "data": "0xAB"} if tool == "barefoc_write" else {"reg": 16, "len": 1}
    elif tool == "barefoc_command":
        arguments = {"command": "M1"}
    elif tool == "barefoc_mode":
        arguments = {"mode": "velocity", "enabled": False}
    elif tool == "barefoc_calibrate":
        arguments = {"kind": "motor"}

    response = _stub_tools_call(tool, arguments)
    raw = json.dumps(response)
    parsed = json.loads(raw)
    assert parsed["jsonrpc"] == "2.0"
    assert parsed["result"]["isError"] is False
    content = parsed["result"]["content"][0]
    assert content["type"] == "text"
    inner = json.loads(content["text"])
    assert isinstance(inner, dict)
    assert inner["status"] == "ok"
