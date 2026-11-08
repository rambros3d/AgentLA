#!/usr/bin/env python3
"""
Unit and integration tests for CAN Bus Analyzer app (can-sniffer) and CAN WebSocket bridge.
Tests manifest schema, UI capabilities, firmware decoupling, and WebSocket routing.
"""

import json
from pathlib import Path
import pytest


def test_can_sniffer_manifest_schema():
    manifest_file = Path("apps/can-sniffer/app.json")

    assert manifest_file.exists(), "apps/can-sniffer/app.json must exist"

    manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
    assert manifest["name"] == "can-sniffer"
    assert "CAN Bus Analyzer" in manifest["title"]
    assert "1.2.0" in manifest["version"]
    assert manifest["author"] == "AgentLA Team"
    assert manifest["category"] == "Automotive"
    assert "has_hardware_can" in manifest.get("requires", [])
    assert manifest["icon"].startswith("data:image/svg+xml")


def test_can_sniffer_html_capabilities():
    html_file = Path("apps/can-sniffer/index.html")
    assert html_file.exists(), "apps/can-sniffer/index.html must exist"

    content = html_file.read_text(encoding="utf-8")

    # 1. Neumorphic dark layout & brand
    assert 'data-theme="dark"' in content
    assert "CAN Bus Analyzer" in content
    assert "agentla-bridge.js" in content

    # 2. WebSocket streaming connection
    assert "/api/v1/can/ws" in content
    assert "ws-badge" in content

    # 3. 120 Ohm termination control
    assert "/api/v1/bridge/can/termination" in content
    assert "btn-term-toggle" in content

    # 4. Real-time sniffer table
    assert "can-table" in content
    assert "can-tbody" in content
    assert "input-filter-id" in content
    assert "chk-autoscroll" in content

    # 5. Frame injector (TX)
    assert "tx-id" in content
    assert "tx-dlc" in content
    assert "tx-data" in content
    assert "btn-send-tx" in content
    assert "chk-periodic" in content

    # 6. CSV logging & export
    assert "btn-export-csv" in content


def test_firmware_can_seeding_and_websocket():
    apps_rs = Path("firmware-rs/src/apps.rs")
    assert apps_rs.exists(), "firmware-rs/src/apps.rs must exist"

    apps_code = apps_rs.read_text(encoding="utf-8")
    assert "seed_default_apps" in apps_code
    # Ensure static HTML strings were removed from ROM
    assert "SERVO_MASTER_INDEX_HTML" not in apps_code
    assert "TERMINAL_INDEX_HTML" not in apps_code

    web_rs = Path("firmware-rs/src/web.rs")
    web_code = web_rs.read_text(encoding="utf-8")
    assert "/api/v1/can/ws" in web_code

    rest_rs = Path("firmware-rs/src/rest.rs")
    rest_code = rest_rs.read_text(encoding="utf-8")
    assert "CanWebSocket" in rest_code
    assert "CanWsCallback" in rest_code


def test_official_app_packages():
    apps_dir = Path("apps")
    assert apps_dir.exists() and apps_dir.is_dir(), "apps must exist"

    # Terminal is promoted to built-in system app and must not exist in apps/
    assert not (apps_dir / "terminal").exists(), "apps/terminal must be removed (it is a built-in system app)"

    # apps/catalog.json is the official app store catalog for user installable apps
    catalog_file = apps_dir / "catalog.json"
    assert catalog_file.exists(), "apps/catalog.json must exist as official app store catalog"

    catalog = json.loads(catalog_file.read_text(encoding="utf-8"))
    app_ids = [a["id"] for a in catalog.get("apps", [])]
    assert "servo-master" in app_ids
    assert "can-sniffer" in app_ids
    assert "terminal" not in app_ids, "Terminal must not be in user app store catalog"

    pkg_dir = apps_dir / "packages"
    assert pkg_dir.exists() and pkg_dir.is_dir(), "apps/packages must exist"
    assert (pkg_dir / "catalog.json").exists(), "apps/packages/catalog.json must exist"

    for app_id in ["servo-master", "can-sniffer"]:
        tar_file = pkg_dir / f"{app_id}.tar"
        assert tar_file.exists(), f"apps/packages/{app_id}.tar must exist"
        assert tar_file.stat().st_size > 0
