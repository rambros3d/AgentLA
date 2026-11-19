"""Runtime smoke test for BareFOC Studio inline JavaScript.

Executes the full inline script of webui/barefoc.html inside a minimal DOM/bridge
stub (Node.js) to catch runtime errors that static syntax checks cannot detect.
Verifies the ODrive-GUI-style shell actually initializes: discovery, parameter
tree, dashboards, multi-axis footer, and the stepped wizard.
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
    assert 'targets: ["sim0:2","sim1:2"]' in out, out  # multi-device, multi-axis
    assert "dashboards: Tuning" in out, out
    assert "footer chips: 4" in out, out
    assert "wizard pages: 19" in out, out
    assert "MOTOR_ERROR_PHASE_RESISTANCE_OUT_OF_RANGE" in out, out  # error decoding
    assert "RUNTIME ERROR" not in out, out
