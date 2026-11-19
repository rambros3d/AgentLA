#!/usr/bin/env python3
"""BareFOC canonical route and WebUI reference test suite.

Verifies the /barefoc canonical route swap, /motor -> /barefoc 301 redirects,
subpath asset fallbacks, canonical hash routing, and nav/dock label updates.
No live device is required: source-level assertions validate the compiled routes.
"""

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent


def _web_rs():
    return (REPO_ROOT / "firmware-rs" / "src" / "web.rs").read_text(encoding="utf-8")


def test_canonical_route_and_redirects():
    web = _web_rs()
    # Canonical /barefoc serves the barefoc.html asset with HTTP 200
    assert '.route("/barefoc", get_service(crate::fs_http::SystemWebFile::new("barefoc.html", "text/html")))' in web
    # /barefoc.html and /motor.html keep serving the asset directly
    assert '.route("/barefoc.html", get_service(crate::fs_http::SystemWebFile::new("barefoc.html", "text/html")))' in web
    assert '.route("/motor.html", get_service(crate::fs_http::SystemWebFile::new("barefoc.html", "text/html")))' in web
    # Aliases redirect 301 to /barefoc
    for alias in ("/motor", "/motor/", "/foc", "/foc/", "/barefoc/"):
        block = f'.route("{alias}", get(|| async move {{'
        assert block in web, f"missing redirect route {alias}"
    # Every redirect Location in the BareFOC section targets /barefoc
    assert web.count('("Location", "/barefoc")') >= 5
    # No redirect to the old /motor canonical target remains
    assert '("Location", "/motor")' not in web


def test_barefoc_subpath_asset_fallbacks():
    web = _web_rs()
    for asset, mime in [
        ("style.css", "text/css"),
        ("daisyui.css", "text/css"),
        ("themes.css", "text/css"),
        ("agentla-bridge.js", "application/javascript"),
    ]:
        assert f'.route("/barefoc/{asset}", get_service(crate::fs_http::SystemWebFile::new("{asset}", "{mime}")))' in web
        # /motor/... compatibility variants are retained
        assert f'.route("/motor/{asset}", get_service(crate::fs_http::SystemWebFile::new("{asset}", "{mime}")))' in web


def test_webui_nav_uses_barefoc():
    for name in ["barefoc.html", "lab.html", "terminal.html", "files.html", "servo.html", "neopixel.html", "sensor.html"]:
        html = (REPO_ROOT / "webui" / name).read_text(encoding="utf-8")
        assert 'href="/motor"' not in html, f"{name} still links to /motor"
        assert 'href="/barefoc"' in html, f"{name} missing /barefoc top-rail link"
        assert ">BareFOC</a>" in html, f"{name} missing BareFOC nav label"


def test_index_hash_routing_and_dock():
    index = (REPO_ROOT / "webui" / "index.html").read_text(encoding="utf-8")
    # Canonical hash route first, aliases retained
    assert "hash === '#/barefoc' || hash === '#/motor' || hash === '#/foc'" in index
    assert "targetUrl = isDemo ? 'barefoc.html' : '/barefoc'" in index
    assert "dockId = 'dock-app-barefoc'" in index
    assert 'id="dock-app-barefoc"' in index
    assert "dock-app-motor" not in index
    assert "function openBarefocApp()" in index
    assert "function openMotorApp()" in index  # backwards-compatible wrapper retained


def test_app_js_and_bridge_hash_checks():
    appjs = (REPO_ROOT / "webui" / "app.js").read_text(encoding="utf-8")
    assert "cleanHash === 'barefoc' || cleanHash === 'motor' || cleanHash === 'foc'" in appjs
    assert "window.location.href = isDemo ? 'barefoc.html' : '/barefoc'" in appjs

    bridge = (REPO_ROOT / "webui" / "agentla-bridge.js").read_text(encoding="utf-8")
    assert "curPath.startsWith('/barefoc') || curPath.startsWith('/motor') || curPath.startsWith('/foc')" in bridge
    assert "document.getElementById('dock-app-barefoc')" in bridge
    assert "dock-app-motor" not in bridge


def test_domain_strings_preserved():
    """Domain/protocol strings must NOT be renamed by the route change."""
    barefoc = (REPO_ROOT / "webui" / "barefoc.html").read_text(encoding="utf-8")
    for token in ["MOTOR_ERROR_", "axis0.motor", "SimulatedMotor", "sel-motor-type", "Calibrate Motor"]:
        assert token in barefoc, f"domain string {token} must remain unchanged"
