"""Runtime smoke test for BareFOC Studio inline JavaScript.

Executes the full inline script of webui/barefoc.html inside a minimal DOM/bridge
stub (Node.js) to catch runtime errors that static syntax checks cannot detect.
Verifies the BareFOC Studio shell initializes: Board Identity, Joint Identity,
MultiBus register maps, 4-Axis motor bank, and the Automated Calibration Wizard.
"""

import shutil
import subprocess
from pathlib import Path

HARNESS_JS = Path(__file__).with_name("barefoc_runtime_harness.js")
HTML = Path(__file__).resolve().parent.parent / "webui" / "barefoc.html"


def test_barefoc_studio_runtime_shell_initializes():
    node = shutil.which("node")
    if not node:
        return
    assert HARNESS_JS.exists() and HTML.exists()
    res = subprocess.run([node, str(HARNESS_JS)], capture_output=True, text=True, timeout=30)
    assert res.returncode == 0, f"Runtime error:\n{res.stderr}\n{res.stdout}"
    out = res.stdout
    assert "RUNTIME OK" in out, out
    assert "hardware_uid: 0x04" in out, out
    assert "board_node_id: 0x10" in out, out
    assert "motor_bank: 0x60" in out, out
    assert "calib_cmd: 0xb0" in out, out
    assert "motors: 4" in out, out
    assert "shoulder_pitch" in out, out
    assert "calib badge armed: ARMED" in out, out
    assert "mechanical origin angle: 0" in out, out
    assert "RUNTIME ERROR" not in out, out
