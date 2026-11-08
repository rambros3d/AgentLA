"""
Unit and asset tests for the In-ROM Device Diagnostics & Developer Console (/device).
"""

import os
import re
import pytest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEVICE_HTML_PATH = os.path.join(REPO_ROOT, "firmware-rs", "assets", "device.html")
RECOVERY_HTML_PATH = os.path.join(REPO_ROOT, "firmware-rs", "assets", "recovery.html")
WEB_RS_PATH = os.path.join(REPO_ROOT, "firmware-rs", "src", "web.rs")
FS_HTTP_RS_PATH = os.path.join(REPO_ROOT, "firmware-rs", "src", "fs_http.rs")
REST_RS_PATH = os.path.join(REPO_ROOT, "firmware-rs", "src", "rest.rs")


def test_device_html_exists_and_is_self_contained():
    assert os.path.exists(DEVICE_HTML_PATH), f"Missing {DEVICE_HTML_PATH}"
    content = open(DEVICE_HTML_PATH, "r", encoding="utf-8").read()

    # Must be valid HTML5
    assert "<!DOCTYPE html>" in content
    assert "<html lang=\"en\">" in content
    assert "<title>AgentLA - Device Diagnostics &amp; Developer Console</title>" in content

    # Zero external CDN or stylesheet dependencies
    assert "<link rel=\"stylesheet\"" not in content, "device.html must use inline styles only"
    assert "https://cdn." not in content, "device.html must not depend on external CDNs"
    assert "http://" not in content, "device.html must not make insecure HTTP calls"

    # Must have inline styling
    assert "<style>" in content
    assert "--bg:#0d1117" in content or "background:" in content

    # Heart-Pulse-2 Red Duotone Solar favicon
    assert "heart-pulse" in content or "%23ef4444" in content
    assert "data:image/svg+xml," in content


def test_device_html_contains_required_sections_and_hooks():
    content = open(DEVICE_HTML_PATH, "r", encoding="utf-8").read()

    # Hardware & Telemetry cards
    assert "AGENTLA // SYSTEM_INFO" in content
    assert "MCU &amp; Silicon" in content
    assert "CORE_CHIP" in content
    assert "CORE_TYPE" in content
    assert "coreChip" in content
    assert "coreType" in content
    assert "Power &amp; Thermal" in content
    assert "Firmware (Dual A/B OTA)" in content
    assert "WebUI &amp; Storage (SYSTEM_FS)" in content
    assert "Pin Diagnostics" in content
    assert "Flash Partitions" in content

    # Removed pin controls, top bootsel button, and obsolete reboot/repo_latest hooks
    assert "PROBE_PIN:" not in content
    assert "BOOTSEL [UF2]" not in content
    assert "REPO_LATEST" not in content

    # Pin Diagnostics pod groups and colored full text states (no bottom legend)
    assert "pin-legend" not in content
    assert "ZONE 0 // POWER &amp; SYSTEM INSTRUMENTATION" in content or "ZONE 0 // POWER & SYSTEM INSTRUMENTATION" in content
    assert "POD 1 // TARGET PROGRAMMER &amp; SWD" in content or "POD 1 // TARGET PROGRAMMER & SWD" in content
    assert "POD 2 // INDUSTRIAL TRANSCEIVERS" in content
    assert "POD 3 // EXTERNAL BUS &amp; ADC" in content or "POD 3 // EXTERNAL BUS & ADC" in content
    assert "PROGRAMMER" in content
    assert "UART" in content
    assert "FREE" in content
    assert "#22d3ee" in content  # Cyan for Programmer
    assert "#c084fc" in content  # Purple for UART
    assert "#fbbf24" in content  # Amber for CAN/Bus
    assert "#4ade80" in content  # Green for Logic Analyzer
    assert "#64748b" in content  # Muted Gray for Free (Hi-Z)

    # FS & Firmware metadata keys
    assert "PARTITION_SIZE" in content
    assert "PARTITION_OFFSET" in content
    assert "FILESYSTEM" in content
    assert "INSTALLED" in content
    assert "UPDATE" in content
    assert "UP TO DATE" in content

    # Endpoints called
    assert "/api/board/manifest" in content
    assert "/api/v1/system/telemetry" in content
    assert "/api/v1/system/firmware/status" in content
    assert "/api/v1/system/firmware/ota" in content
    assert "/api/v1/system/webui/update" in content
    assert "/api/v1/system/firmware/rollback" in content
    assert "/api/v1/system/bootsel" in content
    assert "/api/v1/system/reboot" in content
    assert "/api/v1/system/pins/release_all" in content
    assert "/api/v1/system/pins/toggle" in content
    assert "/api/v1/system/fs/format" in content

    # Browser-mediated GitHub checker
    assert "api.github.com/repos" in content
    assert "Auto Update" in content

    # Dynamic Recovery Banner components
    assert "recoveryBanner" in content
    assert "SYSTEM_FS RECOVERY MODE" in content
    assert "recDropZone" in content
    assert "btnRecAuto" in content
    assert "Auto Update from GitHub" in content
    assert "toggleRecoveryBanner" in content

    # Multi-view Tab Switcher & Panels
    assert "tabHardware" in content
    assert "tabPins" in content
    assert "tabApi" in content
    assert "tabMcp" in content
    assert "viewHardware" in content
    assert "viewPins" in content
    assert "viewApi" in content
    assert "viewMcp" in content
    assert "switchView" in content
    assert "initView" in content

    # REST API Catalog & Interactive Runner
    assert "API_ENDPOINTS" in content
    assert "apiTable" in content
    assert "copyCurl" in content
    assert "tryApi" in content
    assert "apiRunnerOutput" in content

    # MCP Agent Bridge & Schemas
    assert "MCP_TOOLS" in content
    assert "mcp_target_diagnose" in content
    assert "mcp_inspect_firmware" in content
    assert "mcpConfigSnippet" in content
    assert "copyMcpConfig" in content
    assert "executeMcpRpc" in content


def test_firmware_routing_for_device_and_developer_endpoints():
    web_rs = open(WEB_RS_PATH, "r", encoding="utf-8").read()
    rest_rs = open(REST_RS_PATH, "r", encoding="utf-8").read()

    # Canonical and fallback routes
    assert '.route("/device",' in web_rs
    assert '.route("/device.html",' in web_rs
    assert '.route("/device/",' in web_rs
    assert '.route("/device/pins",' in web_rs
    assert '.route("/device/pins/",' in web_rs
    assert '.route("/device/api",' in web_rs
    assert '.route("/device/api/",' in web_rs
    assert '.route("/device/mcp",' in web_rs
    assert '.route("/device/mcp/",' in web_rs

    # Developer REST services registered
    assert '.route_service("/api/v1/system/bootsel", crate::rest::SystemBootsel)' in web_rs
    assert '.route_service("/api/v1/system/reboot", crate::rest::SystemReboot)' in web_rs
    assert '.route_service("/api/v1/system/pins/release_all", crate::rest::SystemPinsReleaseAll)' in web_rs
    assert '.route_service("/api/v1/system/pins/toggle", crate::rest::SystemPinsToggle)' in web_rs
    assert '.route_service("/api/v1/system/fs/format", crate::rest::SystemFsFormat)' in web_rs

    # REST service implementations exist
    assert "pub struct SystemBootsel;" in rest_rs
    assert "pub struct SystemReboot;" in rest_rs
    assert "pub struct SystemPinsReleaseAll;" in rest_rs
    assert "pub struct SystemPinsToggle;" in rest_rs
    assert "pub struct SystemFsFormat;" in rest_rs


def test_recovery_html_exists_and_is_self_contained():
    assert os.path.exists(RECOVERY_HTML_PATH), f"Missing {RECOVERY_HTML_PATH}"
    content = open(RECOVERY_HTML_PATH, "r", encoding="utf-8").read()

    # Must be valid HTML5
    assert "<!DOCTYPE html>" in content
    assert "<html lang=\"en\">" in content
    assert "<title>AgentLA - WebUI Partition Recovery</title>" in content

    # Zero external CDN or stylesheet dependencies
    assert "<link rel=\"stylesheet\"" not in content, "recovery.html must use inline styles only"
    assert "https://cdn." not in content, "recovery.html must not depend on external CDNs"
    assert "http://" not in content, "recovery.html must not make insecure HTTP calls"

    # Must have inline styling
    assert "<style>" in content
    assert "--bg:#09090b" in content or "background:" in content


def test_recovery_html_contains_required_recovery_actions_and_links():
    content = open(RECOVERY_HTML_PATH, "r", encoding="utf-8").read()
    fs_http_rs = open(FS_HTTP_RS_PATH, "r", encoding="utf-8").read()

    # Recovery UI components
    assert "AGENTLA // SYSTEM_FS RECOVERY" in content
    assert "dropZone" in content
    assert "tarInput" in content
    assert "btnUpload" in content
    assert "Flash SYSTEM_FS" in content

    # Endpoints called
    assert "/api/v1/system/webui/update" in content

    # GitHub release puller
    assert "api.github.com/repos" in content
    assert "Download now (RECOMMENDED)" in content or "pullFromGitHub()" in content

    # High-contrast navigation to Developer Diagnostics Console
    assert "/device" in content
    assert "Open Diagnostics Console (/device)" in content or "location.href='/device'" in content

    # Firmware integration in fs_http.rs
    assert 'include_str!("../assets/recovery.html")' in fs_http_rs
    assert "RECOVERY_HTML" in fs_http_rs
