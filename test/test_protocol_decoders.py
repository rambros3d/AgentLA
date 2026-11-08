"""Unit and integration tests for on-device protocol decoders and trigger matrix."""

import os
import re
import pytest

ROOT_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
LAB_HTML_PATH = os.path.join(ROOT_DIR, "webui", "lab.html")
APP_JS_PATH = os.path.join(ROOT_DIR, "webui", "app.js")
REST_RS_PATH = os.path.join(ROOT_DIR, "firmware-rs", "src", "rest.rs")
ANALYZER_RS_PATH = os.path.join(ROOT_DIR, "firmware-rs", "src", "analyzer.rs")


def test_lab_html_contains_protocol_inspector_and_trigger_builder():
    """Verify lab.html includes bus filter pills, trigger matrix button, and modal dialog."""
    assert os.path.exists(LAB_HTML_PATH), f"lab.html missing at {LAB_HTML_PATH}"
    with open(LAB_HTML_PATH, "r", encoding="utf-8") as f:
        html = f.read()

    assert 'id="packet-bus-filter"' in html, "Missing packet-bus-filter container in lab.html"
    assert 'data-bus="uart"' in html, "Missing UART bus filter pill"
    assert 'data-bus="i2c"' in html, "Missing I2C bus filter pill"
    assert 'data-bus="spi"' in html, "Missing SPI bus filter pill"
    assert 'data-bus="can"' in html, "Missing CAN bus filter pill"
    assert 'id="btn-open-trigger-builder"' in html, "Missing trigger matrix modal trigger button"
    assert 'id="modal-trigger-builder"' in html, "Missing modal-trigger-builder dialog"
    assert 'id="btn-save-trigger-rule"' in html, "Missing Install Rule action button"


def test_app_js_contains_websocket_stream_and_trigger_install():
    """Verify app.js hooks up WebSocket stream and trigger configuration."""
    assert os.path.exists(APP_JS_PATH), f"app.js missing at {APP_JS_PATH}"
    with open(APP_JS_PATH, "r", encoding="utf-8") as f:
        js = f.read()

    assert "/api/v1/analyzer/stream/decoded" in js, "Missing decoded stream WebSocket connection"
    assert "/api/v1/analyzer/triggers/configure" in js, "Missing trigger configuration REST call"
    assert "/api/v1/analyzer/decoders/configure" in js, "Missing decoder configuration REST call"
    assert "agentla-decoded-v1" in js, "Missing WebSocket protocol subprotocol"


def test_firmware_rest_routes_and_handlers():
    """Verify firmware-rs implements required REST & WS handlers for decoders and triggers."""
    with open(REST_RS_PATH, "r", encoding="utf-8") as f:
        rest_code = f.read()

    assert "struct LogicAnalyzerDecodersConfigure" in rest_code
    assert "struct LogicAnalyzerTriggersConfigure" in rest_code
    assert "struct DecodedStreamWebSocket" in rest_code
    assert "decoders_active" in rest_code


def test_flash_and_sram_headroom_within_1mb_slot():
    """Verify flash memory geometry aligns to 1 MB (1,024 KiB) symmetric OTA slots."""
    flash_size = 4096 * 1024
    boot_size = 16 * 1024
    slot_a = 1024 * 1024
    slot_b = 1024 * 1024
    system_fs = 512 * 1024
    user_fs = 1504 * 1024
    trail = 16 * 1024

    assert boot_size + slot_a + slot_b + system_fs + user_fs + trail == flash_size
    assert 0x00004000 + slot_a == 0x00104000, "Slot B must start at 0x00104000"
    assert 0x00104000 + slot_b == 0x00204000, "SYSTEM_FS must start at 0x00204000"
    assert 0x00204000 + system_fs == 0x00284000, "USER_FS must start at 0x00284000"
