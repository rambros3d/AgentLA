#!/usr/bin/env python3
"""
Test suite for Native NeoPixel In-ROM System App and MCP Controls (`native-neopixel-system-app`).
Validates:
- In-ROM system asset `webui/neopixel.html` integrity and canonical links.
- Firmware route registration in `firmware-rs/src/web.rs` for /neopixel and /led alias.
- Universal top-rail navigation across canonical workbench pages.
- Floating dock and launcher drawer integration using Solar Bold Duotone Filters icon.
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


def test_neopixel_in_rom_asset():
    html_path = REPO_ROOT / "webui" / "neopixel.html"
    assert html_path.exists(), "webui/neopixel.html must exist as a permanent system asset"

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
    assert "NeoPixel Control" in content

    # Top-rail navigation header
    assert 'class="navbar-center hidden sm:flex items-center gap-1 font-mono-tech text-xs"' in content
    assert 'href="/home"' in content
    assert 'href="/scope"' in content
    assert 'href="/terminal"' in content
    assert 'href="/files"' in content
    assert 'href="/barefoc"' in content
    assert 'href="/servo"' in content
    assert 'href="/neopixel"' in content

    # Active class on NeoPixel
    assert 'href="/neopixel" class="btn btn-ghost btn-xs font-mono-tech uppercase btn-active text-primary bg-primary/10">NeoPixel</a>' in content

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
    ]

    for rel_path in pages:
        p = REPO_ROOT / rel_path
        assert p.exists(), f"{rel_path} must exist"
        content = p.read_text(encoding="utf-8")
        assert "/neopixel" in content or "#/neopixel" in content, f"{rel_path} must link to NeoPixel"


def test_firmware_route_registration():
    web_rs = REPO_ROOT / "firmware-rs" / "src" / "web.rs"
    assert web_rs.exists(), "firmware-rs/src/web.rs must exist"

    content = web_rs.read_text(encoding="utf-8")

    # Route /neopixel
    assert '.route("/neopixel", get_service(crate::fs_http::SystemWebFile::new("neopixel.html", "text/html")))' in content
    assert '.route("/neopixel.html", get_service(crate::fs_http::SystemWebFile::new("neopixel.html", "text/html")))' in content
    assert '.route("/neopixel/",' in content

    # Redirect /led -> /neopixel
    assert '.route("/led",' in content
    assert '.route("/led/",' in content
    assert '("Location", "/neopixel")' in content


def test_dock_and_app_drawer_integration():
    index_html = (REPO_ROOT / "webui" / "index.html").read_text(encoding="utf-8")
    bridge_js = (REPO_ROOT / "webui" / "agentla-bridge.js").read_text(encoding="utf-8")
    app_js = (REPO_ROOT / "webui" / "app.js").read_text(encoding="utf-8")

    # Dock ID
    assert 'id="dock-app-neopixel"' in index_html
    assert 'id="dock-app-neopixel"' in bridge_js

    # Solar Bold Duotone Filters SVG (3 circles, opacity="0.5")
    assert 'viewBox="0 0 24 24"' in index_html
    assert 'M17.5801 10.21' in index_html
    assert 'M17.5801 10.21' in bridge_js

    # Pinned apps default list includes 'neopixel'
    assert "'neopixel'" in bridge_js
    assert "'neopixel'" in app_js

    # Open functions
    assert "openNeoPixelApp" in index_html
    assert "openNeoPixelApp" in app_js

    # Hash routing
    assert "#/neopixel" in index_html


def test_surface_only_active_state_no_dot_indicators():
    files_to_check = [
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
        "webui/neopixel.html",
        "crates/agentla-mcp/src/tools.rs",
        "crates/agentla-mcp/tests/mcp_rpc.rs",
        "firmware-rs/src/mcp.rs",
        "firmware-rs/src/instruments/ws2812.rs",
        "firmware-rs/src/web.rs",
        "tools/pack_webui.py",
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
    assert manifest["capabilities"]["tools"]["count"] == 78, "mcp.json tool count must be exactly 78"

    tools_rs = (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")
    assert "pub static ALL_TOOLS: [Tool; 78]" in tools_rs

    # 6 NeoPixel tools
    for tool_name in [
        "TOOL_NEOPIXEL_FILL",
        "TOOL_NEOPIXEL_SET",
        "TOOL_NEOPIXEL_CLEAR",
        "TOOL_NEOPIXEL_PATTERN",
        "TOOL_NEOPIXEL_ANALYZE",
        "TOOL_NEOPIXEL_TRANSMIT",
    ]:
        assert tool_name in tools_rs


def test_mcp_neopixel_tool_schemas():
    tools_rs = (REPO_ROOT / "crates" / "agentla-mcp" / "src" / "tools.rs").read_text(encoding="utf-8")

    assert '"neopixel_fill"' in tools_rs
    assert '"neopixel_set"' in tools_rs
    assert '"neopixel_clear"' in tools_rs
    assert '"neopixel_pattern"' in tools_rs
    assert '"neopixel_analyze"' in tools_rs
    assert '"neopixel_transmit"' in tools_rs


def test_pack_webui_system_apps():
    pack_py = (REPO_ROOT / "tools" / "pack_webui.py").read_text(encoding="utf-8")
    assert '"neopixel"' in pack_py
    assert 'SYSTEM_APPS = {' in pack_py
