#!/usr/bin/env python3
"""
Test suite for Native Sensor Studio In-ROM Workbench and MCP Controls (`native-sensor-studio`).
Validates:
- In-ROM system asset `webui/sensor.html` integrity, Bento layout, dual-bus switcher, and canonical links.
- Firmware route registration in `firmware-rs/src/web.rs` for /sensor, /sensors, and /bus aliases.
- Universal top-rail navigation across canonical workbench pages.
- Floating dock and launcher drawer integration using Solar Bold Duotone Radar icon.
- Surface-only active state highlights with zero CSS dot indicators.
- Strict zero-emoji compliance across all new/modified assets.
- MCP tool declarations in `crates/agentla-mcp` and `webui/mcp.json` tool capability count (56).
- WebUI packaging inclusion in `tools/pack_webui.py`.
"""

import json
import re
from pathlib import Path
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent


def test_sensor_studio_in_rom_asset():
    html_path = REPO_ROOT / "webui" / "sensor.html"
    assert html_path.exists(), "webui/sensor.html must exist as a permanent system asset"

    content = html_path.read_text(encoding="utf-8")

    # Canonical asset paths
    assert 'href="/daisyui.css"' in content
    assert 'href="/style.css"' in content
    assert 'href="/themes.css"' in content
    assert 'src="/agentla-bridge.js"' in content
    assert 'href="/logo.svg"' in content
    assert 'href="/favicon.svg"' in content

    # Branding
    assert "AgentLA" in content
    assert "Sensor Studio" in content

    # Top-rail navigation header
    assert 'class="navbar-center hidden sm:flex items-center gap-1 font-mono-tech text-xs"' in content
    assert 'href="/home"' in content
    assert 'href="/scope"' in content
    assert 'href="/terminal"' in content
    assert 'href="/files"' in content
    assert 'href="/barefoc"' in content
    assert 'href="/servo"' in content
    assert 'href="/neopixel"' in content
    assert 'href="/sensor"' in content

    # Active class on Sensor
    assert 'href="/sensor" class="btn btn-ghost btn-xs font-mono-tech uppercase btn-active text-primary bg-primary/10">Sensor</a>' in content

    # Dual-bus mode switcher
    assert 'id="tab-mode-i2c"' in content
    assert 'id="tab-mode-spi"' in content

    # 128-address visual scan radar
    assert 'id="radar-matrix-grid"' in content
    assert 'id="btn-scan-bus"' in content

    # Interactive register map
    assert 'id="reg-matrix-grid"' in content
    assert 'bit-btn' in content

    # Live Canvas visualizers
    assert 'id="telemetry-canvas"' in content

    # Emergency safe state
    assert 'id="btn-emergency-safe"' in content
    assert "triggerEmergencySafe" in content


def test_universal_top_rail_navigation():
    pages = [
        "webui/index.html",
        "webui/lab.html",
        "webui/terminal.html",
        "webui/files.html",
        "webui/barefoc.html",
        "webui/servo.html",
        "webui/neopixel.html",
        "webui/sensor.html",
    ]

    for rel_path in pages:
        p = REPO_ROOT / rel_path
        assert p.exists(), f"{rel_path} must exist"
        content = p.read_text(encoding="utf-8")
        assert "/sensor" in content or "#/sensor" in content, f"{rel_path} must link to Sensor"


def test_firmware_route_registration():
    web_rs = REPO_ROOT / "firmware-rs" / "src" / "web.rs"
    assert web_rs.exists(), "firmware-rs/src/web.rs must exist"

    content = web_rs.read_text(encoding="utf-8")

    # Route /sensor
    assert '.route("/sensor", get_service(crate::fs_http::SystemWebFile::new("sensor.html", "text/html")))' in content
    assert '.route("/sensor.html", get_service(crate::fs_http::SystemWebFile::new("sensor.html", "text/html")))' in content
    assert '.route("/sensor/",' in content

    # Redirect /sensors -> /sensor
    assert '.route("/sensors",' in content
    assert '.route("/sensors/",' in content

    # Redirect /bus -> /sensor
    assert '.route("/bus",' in content
    assert '.route("/bus/",' in content
    assert '("Location", "/sensor")' in content


def test_dock_and_app_drawer_integration():
    index_html = (REPO_ROOT / "webui" / "index.html").read_text(encoding="utf-8")
    bridge_js = (REPO_ROOT / "webui" / "agentla-bridge.js").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "webui" / "app.js").read_text(encoding="utf-8")

    # Dock ID
    assert 'id="dock-app-sensor"' in index_html
    assert 'id="dock-app-sensor"' in bridge_js

    # Solar Bold Duotone Radar SVG (circle + arc radar paths)
    assert 'viewBox="0 0 24 24"' in index_html
    assert 'M11.1256 6.82338' in index_html
    assert 'M11.1256 6.82338' in bridge_js

    # Pinned apps default list includes 'sensor'
    assert "'sensor'" in bridge_js
    assert "'sensor'" in app_js

    # Open functions
    assert "openSensorApp" in index_html
    assert "openSensorApp" in app_js

    # Hash routing
    assert "#/sensor" in index_html


def test_surface_only_active_state_no_dot_indicators():
    files_to_check = [
        "webui/sensor.html",
        "webui/neopixel.html",
        "webui/servo.html",
        "webui/index.html",
        "webui/style.css",
        "webui/agentla-bridge.js",
    ]

    for rel_path in files_to_check:
        p = REPO_ROOT / rel_path
        content = p.read_text(encoding="utf-8")
        # Ensure no active dot indicator rules like .dock-icon-btn.active::after or .nav-tab-btn.active::after
        assert not re.search(r"\.dock-icon-btn\.active::after\s*\{[^}]*content:\s*['\"][^'\"]*['\"]", content)
        assert not re.search(r"\.nav-tab-btn\.active::after\s*\{[^}]*content:\s*['\"][^'\"]*['\"]", content)


def test_zero_emojis_rule():
    # Regex matching emoji ranges
    emoji_pattern = re.compile(
        "[\U0001F600-\U0001F64F"  # Emoticons
        "\U0001F300-\U0001F5FF"  # Misc Symbols & Pictographs
        "\U0001F680-\U0001F6FF"  # Transport & Map
        "\U0001F1E0-\U0001F1FF"  # Flags
        "\U00002702-\U000027B0"  # Dingbats
        "\U0001F900-\U0001F9FF"  # Supplemental Symbols and Pictographs
        "\U0001FA00-\U0001FA6F"  # Chess Symbols
        "\U0001FA70-\U0001FAFF"  # Symbols and Pictographs Extended-A
        "]+",
        flags=re.UNICODE,
    )

    checked_files = [
        "webui/sensor.html",
        "crates/agentla-mcp/src/tools.rs",
        "crates/agentla-mcp/tests/mcp_rpc.rs",
        "firmware-rs/src/mcp.rs",
        "firmware-rs/src/web.rs",
        "tools/pack_webui.py",
        "test/test_sensor_studio_app.py",
    ]

    for rel_path in checked_files:
        p = REPO_ROOT / rel_path
        content = p.read_text(encoding="utf-8")
        match = emoji_pattern.search(content)
        assert match is None, f"Emoji forbidden by zero-emoji policy found in {rel_path}: {match.group(0)}"


def test_mcp_manifest_and_tools_count():
    manifest_path = REPO_ROOT / "webui" / "mcp.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))

    assert manifest["name"] == "AgentLA RP2350"
    assert manifest["capabilities"]["tools"]["count"] == 56, "mcp.json tool count must be exactly 56"

    tools_rs = (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")
    assert "pub static ALL_TOOLS: [Tool; 56]" in tools_rs

    # 4 Sensor tools
    for tool_name in [
        "TOOL_SENSOR_SCAN",
        "TOOL_SENSOR_READ_REG",
        "TOOL_SENSOR_WRITE_REG",
        "TOOL_SENSOR_POLL",
    ]:
        assert tool_name in tools_rs


def test_mcp_sensor_tool_schemas():
    tools_rs = (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")

    assert '"sensor_scan"' in tools_rs
    assert '"sensor_read_reg"' in tools_rs
    assert '"sensor_write_reg"' in tools_rs
    assert '"sensor_poll"' in tools_rs


def test_pack_webui_system_apps():
    pack_py = (REPO_ROOT / "tools" / "pack_webui.py").read_text(encoding="utf-8")
    assert '"sensor"' in pack_py
    assert 'SYSTEM_APPS = {' in pack_py
