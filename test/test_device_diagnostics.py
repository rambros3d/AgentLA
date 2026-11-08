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
    assert "WebUI Storage (SYSTEM_FS)" in content
    assert "User Storage (USER_FS)" in content
    assert "Pin Diagnostics" in content
    assert "Flash Partitions" in content

    # Responsive wrapping assertions (no overflow out of cards)
    assert ".row .val{font-weight:600;color:var(--fg);white-space:nowrap}" not in content
    assert "word-break:break-word" in content
    assert "overflow-wrap:anywhere" in content

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
    assert "/api/v1/system/clock" in content

    # Overclocking controls in MCU & Silicon card
    assert "btnClk150" in content
    assert "btnClk300" in content
    assert "btnClk500" in content
    assert "btnApplyClock" in content
    assert "clkCurTag" in content

    # Browser-mediated GitHub checker
    assert "rambros3d/AgentLA" in content
    assert "UPDATE ALL" in content or "UPDATE" in content

    # Unified System Update Hero Card on [ HARDWARE ]
    assert "cardSystemUpdate" in content
    assert "btnUpdateAll" in content
    assert "btnCheckUpdate" in content
    assert "runFullSystemUpdate" in content
    assert "(UP TO DATE)" in content
    assert "(AVAILABLE)" in content
    assert "CHECK or NETWORK ERROR" in content
    assert "updateDropZone" not in content

    # Local Flash & Slot controls placed in relevant cards
    assert "fileFw" in content
    assert "fileWeb" in content
    assert "rollbackFw" in content

    # Redundant recovery banner eliminated
    assert "recoveryBanner" not in content
    assert "recDropZone" not in content
    assert "SYSTEM_FS RECOVERY // WEBUI RESTORE REQUIRED" not in content

    # Multi-view Tab Switcher & Panels (5 tabs)
    assert "tabHardware" in content
    assert "tabStorage" in content
    assert "tabPins" in content
    assert "tabApi" in content
    assert "tabMcp" in content
    assert "viewHardware" in content
    assert "viewStorage" in content
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
    assert '.route("/device/storage",' in web_rs
    assert '.route("/device/storage/",' in web_rs
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


def test_system_fs_corruption_fallback_redirects_to_device():
    fs_http_rs = open(FS_HTTP_RS_PATH, "r", encoding="utf-8").read()

    # Obsolete recovery.html is eliminated in favor of unified /device
    assert not os.path.exists(RECOVERY_HTML_PATH), "recovery.html has been removed; functions are unified into /device"
    assert 'include_str!("../assets/recovery.html")' not in fs_http_rs
    assert "RECOVERY_HTML" not in fs_http_rs

    # HTML fallback routes redirect directly to /device
    assert '("Location", "/device")' in fs_http_rs
    assert '("Location", "/device?recovery=1")' not in fs_http_rs
    assert "StatusCode::FOUND" in fs_http_rs
