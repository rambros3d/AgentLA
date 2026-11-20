"""Static verification for BareFOC Studio contracts, styling, and register maps.

These tests validate the structural contracts in webui/barefoc.html:
Board Identity, Joint Identity, 4-Axis Motor Bank, Automated Calibration Wizard,
DaisyUI 5 styling compliance, zero dot indicators, full-bleed viewport, and zero emojis.
They are pure static checks so they run without a browser or jsdom dependency.
"""

import re
from pathlib import Path

HTML_DIR = Path(__file__).resolve().parent.parent / "webui"
BAREFOC = HTML_DIR / "barefoc.html"


def _content():
    return BAREFOC.read_text(encoding="utf-8")


def _scripts(content):
    return re.findall(r"<script(?![^>]*\bsrc=)[^>]*>(.*?)</script>", content, re.DOTALL)


def _js(content):
    return "\n".join(_scripts(content))


def test_zero_emoji_and_no_dot_indicators():
    content = _content()
    assert not re.findall(r"[\U00010000-\U0010ffff]", content), "no emojis allowed"
    assert ".nav-tab-btn::after," in content
    assert ".motor-tab-btn::after," in content
    assert "display: none !important;" in content


def test_all_inline_scripts_are_syntactically_valid():
    import shutil
    import subprocess
    import tempfile

    node = shutil.which("node")
    if not node:
        return
    for idx, src in enumerate(_scripts(_content())):
        with tempfile.NamedTemporaryFile("w", suffix=".js", delete=False) as tmp:
            tmp.write(src)
            path = tmp.name
        try:
            res = subprocess.run([node, "-c", path], capture_output=True, text=True)
            assert res.returncode == 0, f"script {idx}: {res.stderr}"
        finally:
            Path(path).unlink(missing_ok=True)


def test_board_and_joint_identity_components():
    content = _content()
    # Board Identity
    assert 'id="board-uid-pill"' in content
    assert 'id="board-uid-val"' in content
    assert 'id="input-board-name"' in content
    assert 'id="input-board-node-id"' in content
    assert 'id="btn-save-board-identity"' in content

    # Motor Selector Tabs with dynamic joint names
    assert 'id="motor-tab-0"' in content
    assert 'id="motor-tab-3"' in content

    # Joint Identity Controls
    assert 'id="joint-identity-controls"' in content
    assert 'id="inp-joint-addr"' in content
    assert 'id="inp-joint-name"' in content
    assert 'id="btn-apply-joint-identity"' in content


def test_multibus_identity_and_motor_bank_registers():
    js = _js(_content())
    # MultiBus Identity offsets
    assert "HARDWARE_UID: 0x04" in js
    assert "BOARD_NODE_ID: 0x10" in js
    assert "BOARD_NAME: 0x11" in js
    assert "JOINT_BASE: 0x20" in js
    assert "JOINT0_ADDR: 0x20" in js
    assert "JOINT0_NAME: 0x21" in js
    assert "JOINT3_ADDR: 0x50" in js
    assert "JOINT3_NAME: 0x51" in js

    # 4-Axis Motor Bank offsets
    assert "MOTOR_BANK_BASE: 0x60" in js
    assert "MOTOR_BANK_STRIDE: 0x20" in js
    assert "MOTOR_POLE_PAIRS_OFFSET: 0x00" in js
    assert "MOTOR_ZERO_ELEC_OFFSET: 0x0E" in js
    assert "MOTOR_SENSOR_DIR_OFFSET: 0x12" in js

    # Calibration commands
    assert "CALIB_CMD: 0xB0" in js
    assert "ALIGN: 0x01" in js
    assert "SET_ORIGIN: 0x04" in js


def test_calibration_wizard_and_arm_badges():
    content = _content()
    # Arm status badge
    assert 'id="badge-arm-status"' in content
    assert "DISARMED_SAFE" in content

    # Calibration telemetry readouts
    assert 'id="calib-elec-zero-val"' in content
    assert 'id="calib-sensor-dir-val"' in content
    assert 'id="calib-commit-status"' in content

    # Calibration action buttons
    assert 'id="btn-calib-align"' in content
    assert 'id="btn-calib-origin"' in content
    assert "1. Run Electrical ALIGN" in content
    assert "Set Mechanical Origin" in content


def test_viewport_geometry_and_daisyui():
    content = _content()
    assert "calc(100vh - 48px)" in content
    assert "vue" not in content.lower()
    assert "react" not in content.lower()
    assert 'src="/agentla-bridge.js"' in content
    assert 'href="/daisyui.css"' in content
