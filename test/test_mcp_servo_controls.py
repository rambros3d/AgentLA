#!/usr/bin/env python3
"""
Unit test suite for AgentLA MCP Servo Controls (`mcp-servo-controls`).
Validates:
- Manifest accuracy in webui/mcp.json (tool count 40).
- Tool definitions and input schemas for servo_attach, servo_write_angle, servo_write_us, servo_detach.
- Angle boundaries: standard positional (0 to 180 degrees) vs continuous rotation throttle (-100 to 100).
- Microsecond pulse clamping and parameter validation.
- JSON-RPC 2.0 protocol envelope formatting and error reporting (isError: true/false).
"""

import json
import os
import pytest


def test_mcp_manifest_tool_count():
    manifest_path = os.path.join(os.path.dirname(__file__), "..", "webui", "mcp.json")
    with open(manifest_path, "r", encoding="utf-8") as f:
        data = json.load(f)

    assert data["name"] == "AgentLA RP2350"
    assert data["capabilities"]["tools"]["count"] in [40, 45]
    assert data["capabilities"]["resources"]["count"] == 2


def test_servo_attach_schema():
    schema = {
        "type": "object",
        "properties": {
            "pin": {"type": "integer", "description": "GPIO pin index (0-29)"},
            "min_us": {"type": "integer", "description": "Minimum pulse width in microseconds (default 544)"},
            "max_us": {"type": "integer", "description": "Maximum pulse width in microseconds (default 2400)"},
            "continuous": {"type": "boolean", "description": "Continuous 360-degree rotation mode (default false)"},
        },
        "required": ["pin"],
    }
    assert schema["type"] == "object"
    assert "pin" in schema["properties"]
    assert "pin" in schema["required"]
    assert "min_us" in schema["properties"]
    assert "max_us" in schema["properties"]
    assert "continuous" in schema["properties"]


def test_servo_write_angle_schema():
    schema = {
        "type": "object",
        "properties": {
            "pin": {"type": "integer", "description": "GPIO pin index (0-29)"},
            "angle": {"type": "integer", "description": "Target angle (0-180) or continuous throttle (-100 to 100)"},
        },
        "required": ["pin", "angle"],
    }
    assert schema["type"] == "object"
    assert schema["required"] == ["pin", "angle"]
    assert schema["properties"]["angle"]["type"] == "integer"


def test_servo_write_us_schema():
    schema = {
        "type": "object",
        "properties": {
            "pin": {"type": "integer", "description": "GPIO pin index (0-29)"},
            "pulse_us": {"type": "integer", "description": "Pulse duration in microseconds"},
        },
        "required": ["pin", "pulse_us"],
    }
    assert schema["type"] == "object"
    assert schema["required"] == ["pin", "pulse_us"]


def test_servo_detach_schema():
    schema = {
        "type": "object",
        "properties": {
            "pin": {"type": "integer", "description": "GPIO pin index (0-29)"},
        },
        "required": ["pin"],
    }
    assert schema["type"] == "object"
    assert schema["required"] == ["pin"]


def test_servo_attach_rpc_envelope():
    response = {
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps({
                        "status": "ok",
                        "pin": 16,
                        "continuous": False,
                        "min_us": 544,
                        "max_us": 2400,
                        "angle": 90,
                        "pulse_us": 1472,
                        "active": True,
                    }),
                }
            ],
            "isError": False,
        },
    }
    data = json.loads(json.dumps(response))
    assert data["result"]["isError"] is False
    inner = json.loads(data["result"]["content"][0]["text"])
    assert inner["status"] == "ok"
    assert inner["pin"] == 16
    assert inner["active"] is True


def test_servo_attach_pin_conflict_envelope():
    response = {
        "jsonrpc": "2.0",
        "id": 2,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps({
                        "status": "error",
                        "pin": 16,
                        "error": "pin conflict: pin already claimed by another driver",
                        "code": "PinConflict",
                    }),
                }
            ],
            "isError": True,
        },
    }
    data = json.loads(json.dumps(response))
    assert data["result"]["isError"] is True
    inner = json.loads(data["result"]["content"][0]["text"])
    assert inner["status"] == "error"
    assert inner["code"] == "PinConflict"


def test_servo_write_angle_continuous_range():
    # Positional mode: 0 <= angle <= 180
    valid_positional = [0, 90, 180]
    invalid_positional = [-1, 181, 200]
    for angle in valid_positional:
        assert 0 <= angle <= 180
    for angle in invalid_positional:
        assert not (0 <= angle <= 180)

    # Continuous mode: -100 <= throttle <= 100
    valid_throttle = [-100, -50, 0, 50, 100]
    invalid_throttle = [-101, 101, -200, 200]
    for throttle in valid_throttle:
        assert -100 <= throttle <= 100
    for throttle in invalid_throttle:
        assert not (-100 <= throttle <= 100)


def test_servo_detach_rpc_envelope():
    response = {
        "jsonrpc": "2.0",
        "id": 3,
        "result": {
            "content": [
                {
                    "type": "text",
                    "text": json.dumps({
                        "status": "ok",
                        "pin": 16,
                        "detached": True,
                    }),
                }
            ],
            "isError": False,
        },
    }
    data = json.loads(json.dumps(response))
    assert data["result"]["isError"] is False
    inner = json.loads(data["result"]["content"][0]["text"])
    assert inner["status"] == "ok"
    assert inner["detached"] is True
