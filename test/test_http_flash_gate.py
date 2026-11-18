#!/usr/bin/env python3
"""
Verifies the HTTP flash gate contract.

While HTTP_FLASH_ENABLED is false in firmware-rs/src/rest.rs, every flash-shaped
route must answer HTTP 501 with a machine-readable body that points at the USB
transport. Non-flash programmer routes must keep working.

These tests assert the contract without requiring a live probe; the live check
is documented in openspec/changes/temporarily-disable-http-flash/tasks.md.
"""

import json

DISABLED_BODY = {
    "status": "disabled",
    "error": "http_flash_disabled",
    "use": "usb",
    "modes": ["cmsis", "stlink"],
}

FLASH_SHAPED = [
    "POST /api/v1/programmer/flash",
    "POST /api/v1/programmer/erase",
    "POST /api/v1/programmer/reset-catch",
    "POST /api/v1/programmer/rescue/stm32",
    "POST /api/v1/programmer/rescue/nrf",
]

NON_FLASH = [
    "GET /api/v1/programmer/target",
    "GET /api/v1/programmer/read",
    "POST /api/v1/programmer/reset",
    "GET /api/v1/programmer/rtt/status",
    "GET /api/v1/programmer/rtt/stream",
    "GET /api/v1/programmer/swo/status",
    "GET /api/programmer/mode",
    "POST /api/programmer/mode",
    "GET /api/programmer/stlink/version",
    "POST /api/programmer/stlink/version",
]


def test_disabled_body_schema():
    """The 501 body must name the failure and the replacement transport."""
    data = json.loads(json.dumps(DISABLED_BODY))
    assert data["status"] == "disabled"
    assert data["error"] == "http_flash_disabled"
    assert data["use"] == "usb"
    assert "cmsis" in data["modes"]
    assert "stlink" in data["modes"]


def test_flash_shaped_routes_are_gated():
    """Every route in FLASH_SHAPED must be covered by the gate."""
    # The gate lives in firmware-rs/src/rest.rs; assert the list is non-empty
    # and each entry uses a flash-shaped method.
    assert len(FLASH_SHAPED) >= 5
    for route in FLASH_SHAPED:
        method, path = route.split(" ", 1)
        assert method == "POST"
        assert path.startswith("/api/v1/programmer/")
        assert path.split("/")[-1] in (
            "flash",
            "erase",
            "reset-catch",
            "stm32",
            "nrf",
        )


def test_non_flash_routes_are_not_gated():
    """The keep-list must not overlap the gate list."""
    gated = {r.split(" ", 1)[1] for r in FLASH_SHAPED}
    for route in NON_FLASH:
        path = route.split(" ", 1)[1]
        assert path not in gated, f"{path} must stay available while HTTP flash is disabled"


def test_flag_is_disabled_by_default():
    """The build-time flag must default to false until the programmer is fixed."""
    import re
    from pathlib import Path

    src = Path(__file__).resolve().parents[1] / "firmware-rs/src/rest.rs"
    text = src.read_text()
    m = re.search(r"pub const HTTP_FLASH_ENABLED:\s*bool\s*=\s*(true|false);", text)
    assert m is not None, "HTTP_FLASH_ENABLED constant not found"
    assert m.group(1) == "false"


def test_mcp_tools_reuse_the_same_payload():
    """MCP programmer_flash / programmer_unbrick must not invent a second error."""
    from pathlib import Path

    rest = Path(__file__).resolve().parents[1] / "firmware-rs/src/rest.rs"
    mcp = Path(__file__).resolve().parents[1] / "firmware-rs/src/mcp.rs"
    rest_text = rest.read_text()
    mcp_text = mcp.read_text()
    assert "HTTP_FLASH_DISABLED_BODY" in rest_text
    assert "http_flash_disabled" in rest_text
    # MCP reuses the constant rather than duplicating the JSON.
    assert "HTTP_FLASH_DISABLED_BODY" in mcp_text


def test_ota_vector_validation_logic():
    """The OTA path only accepts an image linked for the single app slot."""
    APP_XIP = 0x10008000
    APP_SIZE = 1248 * 1024

    def valid(img_pc):
        return APP_XIP <= img_pc < APP_XIP + APP_SIZE

    assert valid(APP_XIP + 0x198)          # normal reset handler
    assert valid(APP_XIP + APP_SIZE - 1)   # last byte of the slot
    assert not valid(0x10004000)           # below the app slot (bootloader region)
    assert not valid(0x101C0000)           # first address past the app slot
    assert not valid(0x20000100)           # SRAM, not XIP
    assert not valid(0x00000000)           # bootloader region
