"""Static verification for the BareFOC Studio <-> ODrive GUI parity change.

These tests validate the structural contracts in webui/barefoc.html that deliver
feature parity with the ODrive GUI reference: discovery, parameter explorer,
configurable dashboards, stepped wizard, arbitrary-variable scope, and the
remediation of previously dead controls. They are pure static checks so they
run without a browser or jsdom dependency.
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


def test_studio_store_and_event_bus():
    js = _js(_content())
    assert "class StudioStore" in js
    for member in ["subscribe(", "commit(", "on(", "emit(", "dispatch("]:
        assert member in js, f"StudioStore missing {member}"
    assert "const studio = new StudioStore(state)" in js
    assert "studio.domains.scope = scope" in js


def test_discovery_service_and_multi_axis_simulator():
    js = _js(_content())
    assert "class DiscoveryService" in js
    assert "async discover()" in js
    # Two simulated targets with two axes each
    assert "Virtual ODrive (SIM)" in js
    assert "Virtual BareFOC (SIM)" in js
    assert "axes: ['axis0','axis1']" in js
    assert "function renderFooter" in js
    assert "studio-shell-footer" in js
    assert "function decodeErrors" in js
    assert "const ERROR_DICT" in js
    assert "const CALIBRATION_MASK" in js
    assert "setInterval(()=>{ if (STUDIO_V2) renderFooter(); }, 1000)" in js


def test_parameter_explorer_and_generic_controls():
    js = _js(_content())
    assert "class ParameterExplorer" in js
    for fn in ["fromFibreSchema", "fromMultiBus", "fromCommander"]:
        assert fn in js, f"ParameterExplorer missing {fn}"
    assert "makeNode(" in js
    assert "function renderParamTree" in js
    assert "function renderControlWidget" in js
    assert "function renderActionWidget" in js
    for t in ["'function'", "'enum'", "'slider'", "'bool'"]:
        assert t in js
    assert "inspectSchema()" in js


def test_dashboard_builder_and_import_export():
    js = _js(_content())
    assert "function newDashboard" in js
    assert "function defaultTuningDashboard" in js
    assert "function addWidgetToDash" in js
    assert "function renderDashboard" in js
    assert "function exportDashboard" in js
    assert "function importDashboardFile" in js
    assert "function exportTargetConfig" in js
    assert "function importConfigFile" in js
    assert "'Export dashboard'" in js
    assert "'Import target config'" in js
    # Default Tuning workflow widgets
    assert "vel_integrator_gain" in js
    assert "requested_state" in js
    assert "save_configuration" in js


def test_wizard_pages_calibration_and_diff():
    js = _js(_content())
    assert "function buildWizardPages" in js
    assert "function renderWizard" in js
    assert "function applyWizardConfig" in js
    # Page graph coverage
    for key in ["target", "supply", "motor_", "encoder_", "control_", "misc_", "review"]:
        assert key in js, f"wizard page {key} missing"
    # Hall encoder CPR derivation and calibration gating
    assert "6*pp" in js or "6 * pp" in js
    assert "Calibrate Motor" in js
    assert "Calibrate Encoder" in js
    assert "wizardState.choiceMade[page.key]" in js
    assert "diff-old" in js and "diff-new" in js
    assert "CALIBRATION_MASK" in js


def test_arbitrary_waveform_plotting():
    js = _js(_content())
    assert "studio.domains.sampledProperties" in js
    assert "function refreshTraceSelectors" in js
    assert "function registerSampled" in js
    assert "getSeries(" in js
    assert "function readSampledValue" in js
    # Retained scope capabilities
    for token in [
        "btn-scope-export", "btn-timebase", "scope.autoScale",
        "scope.frozen", "scope.crosshair", "stepBenchmark",
    ]:
        assert token in js, f"scope capability {token} missing"


def test_stub_remediation():
    js = _js(_content())
    assert "btn-sync-tuning" in js
    assert "inp-supply-v" in js
    assert "inp-voltage-limit" in js
    assert "inp-velocity-limit" in js
    assert "wireScopeCrosshair" in js
    assert "wirePowerGuard" in js
    assert "wireFaultBadges" in js
    assert "guardAlignmentApply" in js


def test_no_framework_or_new_build_artifact():
    content = _content()
    assert "vue" not in content.lower()
    assert "react" not in content.lower()
    assert 'src="/agentla-bridge.js"' in content
    assert 'href="/daisyui.css"' in content
