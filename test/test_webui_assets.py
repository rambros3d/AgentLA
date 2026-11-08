#!/usr/bin/env python3
"""
Unit tests for AgentLA WebUI Assets, Themes, Routing Invariants,
and Floating Action Button Bridge.
"""

from pathlib import Path
import re
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
HTML_DIR = REPO_ROOT / "webui"


def test_theme_compiler_root_cascade_order():
    """Verify themes.css emits :root fallback at the very top before any specific theme rules."""
    themes_css_path = HTML_DIR / "themes.css"
    assert themes_css_path.exists(), "themes.css must exist"
    content = themes_css_path.read_text(encoding="utf-8")

    # 1. :root must appear before any specific [data-theme=...] selector
    root_idx = content.find(":root {")
    assert root_idx != -1, ":root fallback block must be defined in themes.css"

    caramel_idx = content.find('[data-theme="caramellatte"]')
    emerald_idx = content.find('[data-theme="emerald"]')
    forest_idx = content.find('[data-theme="forest"]')

    assert caramel_idx != -1, "caramellatte theme rule missing"
    assert emerald_idx != -1, "emerald theme rule missing"
    assert forest_idx != -1, "forest theme rule missing"

    assert root_idx < caramel_idx, ":root block must precede [data-theme=caramellatte]"
    assert root_idx < emerald_idx, ":root block must precede [data-theme=emerald]"
    assert root_idx < forest_idx, ":root block must precede [data-theme=forest]"

    # 2. Forest rule should not include :root selector in its theme declaration
    forest_match = re.search(r'([^\n]+)\{\s*color-scheme:\s*dark;\s*--color-base-100:[^}]+\}', content)
    assert forest_match is not None


def test_lab_html_clean_header_and_landing_setting():
    """Verify lab.html (Workbench SPA) has clean top bar (no redundant launcher pills) and landing page selector."""
    lab_html_path = HTML_DIR / "lab.html"
    assert lab_html_path.exists(), "webui/lab.html must exist"
    content = lab_html_path.read_text(encoding="utf-8")

    # 1. Redundant nav pill removed
    assert '<nav class="header-nav-links"' not in content, "Redundant nav pill container must be removed"
    assert 'class="preset-pill">App Launcher</a>' not in content

    # 2. Brand logo acts as link to /home
    assert ('href="/home"' in content or 'href="/home/"' in content), "Brand logo must link to /home"
    assert 'id="brand-logo"' in content

    # 3. Settings modal includes landing page selector with canonical routes
    assert 'id="settings-default-home-route-select"' in content, "Landing page selector must be present"
    assert 'value="/home"' in content
    assert 'value="/scope"' in content
    assert 'value="/debug"' in content
    assert 'value="/adapter"' in content
    assert 'value="/pins"' in content
    assert 'value="/agent"' in content
    assert 'value="/terminal"' in content
    assert 'value="/lab"' not in content, "Legacy /lab route must be removed"
    assert 'value="/apps"' not in content, "Legacy /apps route must be removed"

    # 4. Uses relative asset links compatible with both firmware and GitHub Pages
    assert 'href="style.css"' in content
    assert 'href="themes.css"' in content
    assert 'src="app.js"' in content


def test_index_html_unified_os_launcher():
    """Verify index.html (Launcher & Catalog) has DaisyUI styling, squircle grid, spotlight search, and shared settings modal."""
    index_html_path = HTML_DIR / "index.html"
    assert index_html_path.exists(), "webui/index.html must exist"
    content = index_html_path.read_text(encoding="utf-8")

    # 1. No legacy #0d1117 block
    assert "background-color: var(--bg-canvas" in content
    assert 'href="style.css"' in content
    assert 'href="themes.css"' in content

    # 2. Core Apps in launcher
    assert "Scope" in content
    assert "Adapter" in content
    assert "Debugger" in content
    assert "Pin Control" in content
    assert "Console" in content
    assert 'onclick="openLab(\'scope\')"' in content

    # 3. Spotlight search bar
    assert 'id="spotlight-search"' in content
    assert "spotlight-input" in content

    # 4. Segmented storage bar
    assert 'id="storage-bar"' in content
    assert "seg-rom" in content
    assert "seg-apps" in content
    assert "seg-free" in content

    # 5. Shared settings modal
    assert 'id="modal-settings"' in content
    assert 'id="settings-default-home-route-select"' in content
    assert 'id="btn-header-settings"' in content


def test_agentla_bridge_fab_asset():
    """Verify agentla-bridge.js implements floating glass dock, suppression, and app navigation."""
    bridge_path = HTML_DIR / "agentla-bridge.js"
    assert bridge_path.exists()
    content = bridge_path.read_text(encoding="utf-8")

    assert "floating-glass-dock" in content or "dock-pinned-apps" in content
    assert "no_launcher" in content, "Must support ?no_launcher=1 suppression"
    assert "AgentLANoLauncher" in content, "Must support window.AgentLANoLauncher suppression"
    assert "homeUrl" in content or "/apps" in content, "Must navigate to apps"
    style_path = HTML_DIR / "style.css"
    assert style_path.exists()
    style_content = style_path.read_text(encoding="utf-8")
    assert "backdrop-filter" in style_content, "Must have backdrop-filter blur in style.css"


def test_pure_daisyui_stylesheet_and_token_invariants():
    """Verify style.css completely eliminates neumorphic shadows/rim highlights, drafting grid, and adds pure DaisyUI classes."""
    style_css_path = HTML_DIR / "style.css"
    assert style_css_path.exists()
    content = style_css_path.read_text(encoding="utf-8")

    # 1. Zero neumorphic and specular bevel tokens
    assert "--card-neumorphic-shadow" not in content, "Lingering neumorphic shadow token found in style.css"
    assert "--card-rim-highlight" not in content, "Lingering card rim highlight token found in style.css"
    assert "body::before" not in content, "Lingering body noise overlay pseudo-element found in style.css"
    assert "var(--grid-line)" not in content, "Lingering drafting grid reference found in style.css"

    # 2. Pure DaisyUI component classes present
    assert ".navbar" in content, "DaisyUI .navbar component class missing in style.css"
    assert ".navbar-start" in content, "DaisyUI .navbar-start missing in style.css"
    assert ".navbar-end" in content, "DaisyUI .navbar-end missing in style.css"
    assert ".card-border" in content, "DaisyUI .card-border missing in style.css"
    assert ".stats" in content, "DaisyUI .stats component class missing in style.css"
    assert ".stat" in content, "DaisyUI .stat component class missing in style.css"
    assert ".stat-title" in content, "DaisyUI .stat-title missing in style.css"
    assert ".stat-value" in content, "DaisyUI .stat-value missing in style.css"
    assert ".stat-desc" in content, "DaisyUI .stat-desc missing in style.css"
    assert ".table-zebra" in content, "DaisyUI .table-zebra modifier class missing in style.css"
    assert ".table-pin-rows" in content, "DaisyUI .table-pin-rows modifier class missing in style.css"
    assert ".mockup-code" in content, "DaisyUI .mockup-code component class missing in style.css"
    assert ".progress" in content, "DaisyUI .progress component class missing in style.css"
    assert ".status" in content, "DaisyUI .status indicator dot class missing in style.css"

    # 3. Comprehensive layout utility layer present
    assert ".grid" in content, "CSS Grid utility missing in style.css"
    assert ".grid-cols-4" in content, ".grid-cols-4 missing in style.css"
    assert ".md\\:grid-cols-2" in content, "Responsive md:grid-cols-2 missing in style.css"
    assert ".gap-4" in content, ".gap-4 missing in style.css"
    assert ".flex-col" in content, ".flex-col missing in style.css"
    assert ".items-center" in content, ".items-center missing in style.css"
    assert ".justify-between" in content, ".justify-between missing in style.css"
    assert ".p-4" in content, ".p-4 missing in style.css"
    assert ".w-full" in content, ".w-full missing in style.css"


def test_daisyui_bundle_distribution_and_linking():
    """Verify daisyui.css bundle exists, is linked by html files and user apps, and routed in firmware."""
    daisyui_path = HTML_DIR / "daisyui.css"
    assert daisyui_path.exists(), "daisyui.css must exist in webui/"
    content = daisyui_path.read_text(encoding="utf-8")
    assert "daisyUI" in content

    # Check routing in web.rs
    web_rs = REPO_ROOT / "firmware-rs" / "src" / "web.rs"
    assert web_rs.exists()
    web_content = web_rs.read_text(encoding="utf-8")
    assert '"/daisyui.css"' in web_content, "web.rs must route /daisyui.css"

    # Check user apps link daisyui.css
    for app in ["servo-master", "can-sniffer"]:
        app_html = REPO_ROOT / "apps" / app / "index.html"
        assert app_html.exists()
        assert 'href="/daisyui.css"' in app_html.read_text(encoding="utf-8"), f"{app} must link /daisyui.css"


def test_lab_html_daisyui_markup_components():
    """Verify lab.html (Workbench SPA) adopts DaisyUI navbar, telemetry stats, table zebra, and mockup-code."""
    lab_html_path = HTML_DIR / "lab.html"
    assert lab_html_path.exists(), "webui/lab.html must exist"
    content = lab_html_path.read_text(encoding="utf-8")

    assert "navbar" in content, "lab.html top bar must use DaisyUI .navbar"
    assert "navbar-start" in content, "lab.html navbar must include .navbar-start"
    assert "navbar-end" in content, "lab.html navbar must include .navbar-end"
    assert "scope-telemetry-stats" in content, "lab.html scope telemetry container missing"
    assert "stats-horizontal" in content, "lab.html scope telemetry must use DaisyUI .stats-horizontal"
    assert "stat-title" in content, "lab.html scope telemetry must use .stat-title"
    assert "stat-value" in content, "lab.html scope telemetry must use .stat-value"
    assert "table-zebra" in content, "lab.html packet/log table must use DaisyUI .table-zebra"
    assert "mockup-code" in content, "lab.html terminal display must use DaisyUI .mockup-code"


def test_app_js_dynamic_canvas_token_binding():
    """Verify app.js implements dynamic CSS token caching and theme change listener for canvases."""
    app_js_path = HTML_DIR / "app.js"
    assert app_js_path.exists()
    content = app_js_path.read_text(encoding="utf-8")

    assert "getCanvasPalette" in content, "app.js must provide getCanvasPalette() helper"
    assert "invalidateCanvasPalette" in content, "app.js must provide invalidateCanvasPalette() helper"
    assert "agentla-theme-changed" in content, "app.js must listen for and dispatch agentla-theme-changed"


def test_modal_hidden_by_default_and_no_bogus_selector():
    """Verify style.css does not have bogus dialog selectors that permanently open div modals, and index.html hides modals initially."""
    style_css_path = HTML_DIR / "style.css"
    content = style_css_path.read_text(encoding="utf-8")

    assert ":not(dialog:not(.modal-open))" not in content, "Bogus selector found in style.css"
    assert ".modal {" in content

    launcher_html_path = HTML_DIR / "index.html"
    launcher_content = launcher_html_path.read_text(encoding="utf-8")
    assert 'id="modal-settings"' in launcher_content
    assert 'style="display: none;"' in launcher_content


def test_landing_page_light_theme_tokens_and_syntax():
    """Verify web/landing.css defines light mode tokens and has no orphan declarations."""
    landing_css_path = REPO_ROOT / "website" / "landing.css"
    assert landing_css_path.exists()
    content = landing_css_path.read_text(encoding="utf-8")

    assert '[data-theme="emerald"]' in content
    assert '--color-base-100: oklch(100% 0 0);' in content
    assert '--color-base-200: oklch(93% 0 0);' in content
    assert 'display: block;\n}\n    --accent-green' not in content, "Orphan syntax error in landing.css"


def test_cross_page_theme_synchronization():
    """Verify all pages (workbench, launcher, user apps) implement multi-tab/iframe theme synchronization."""
    app_js = (HTML_DIR / "app.js").read_text(encoding="utf-8")
    launcher_html = (HTML_DIR / "index.html").read_text(encoding="utf-8")
    bridge_js = (HTML_DIR / "agentla-bridge.js").read_text(encoding="utf-8")
    servo_path = HTML_DIR.parent / "apps" / "servo-master" / "index.html"
    if not servo_path.exists():
        servo_path = HTML_DIR / "apps" / "servo-master" / "index.html"
    servo_index = servo_path.read_text(encoding="utf-8")

    # 1. All layers listen to storage events
    assert "window.addEventListener('storage'" in app_js, "app.js must listen to storage events"
    assert "window.addEventListener('storage'" in launcher_html, "index.html must listen to storage events"
    assert "window.addEventListener('storage'" in bridge_js, "agentla-bridge.js must listen to storage events"

    # 2. All layers use BroadcastChannel('agentla-theme-channel')
    assert "agentla-theme-channel" in app_js, "app.js must use agentla-theme-channel"
    assert "agentla-theme-channel" in launcher_html, "index.html must use agentla-theme-channel"
    assert "agentla-theme-channel" in bridge_js, "agentla-bridge.js must use agentla-theme-channel"

    # 3. User app integration
    assert '<script src="/agentla-bridge.js"></script>' in servo_index, "servo-master must include agentla-bridge.js"
    assert 'data-theme="forest"' in servo_index, "servo-master must default to forest theme"


def test_mock_bridge_in_webui_and_excluded_from_pack(tmp_path):
    """Verify mock-bridge.js lives in webui/ for demo simulation, but is strictly filtered out of LittleFS packaging."""
    mock_bridge = HTML_DIR / "mock-bridge.js"
    assert mock_bridge.exists(), "mock-bridge.js must exist directly in webui/ for standalone simulation"
    assert "simState" in mock_bridge.read_text(encoding="utf-8")

    import importlib.util
    pack_script = REPO_ROOT / "tools" / "pack_webui.py"
    spec = importlib.util.spec_from_file_location("pack_webui_mod", pack_script)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    out_img = tmp_path / "test.img"
    out_uf2 = tmp_path / "test.uf2"
    out_tar = tmp_path / "test.tar"

    result = mod.pack_webui(
        web_dir=HTML_DIR,
        output_img=out_img,
        output_uf2=out_uf2,
        output_tar=out_tar,
        include_user_apps=False,
    )

    packed_files = result["asset_names"]
    assert "mock-bridge.js" not in packed_files, "mock-bridge.js MUST NOT be packaged into LittleFS image"
    assert not any("mock" in f for f in packed_files), "No mock simulation files allowed in LittleFS"


def test_build_portal_staging_pipeline(tmp_path):
    """Verify build_portal.py cleanly stages website landing page and webui demo without symlinks."""
    import importlib.util
    build_script = REPO_ROOT / "tools" / "build_portal.py"
    assert build_script.exists()
    spec = importlib.util.spec_from_file_location("build_portal_mod", build_script)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)

    out_site = tmp_path / "portal_site"
    res = mod.stage_portal(REPO_ROOT, out_site, clean=True, board="demo")

    assert (out_site / "index.html").exists(), "Landing page must be at root"
    assert (out_site / "landing.css").exists()
    assert (out_site / "landing.js").exists()
    assert (out_site / ".nojekyll").exists()

    # Demo directory checks
    demo_dir = out_site / "demo"
    assert (demo_dir / "index.html").exists(), "Launcher catalog must be in demo/"
    assert (demo_dir / "lab.html").exists(), "WebUI workbench must be in demo/"
    assert (demo_dir / "terminal.html").exists(), "Serial terminal must be in demo/"
    assert (demo_dir / "mock-bridge.js").exists(), "mock-bridge.js must be in demo/"
    assert (demo_dir / "board.json").exists(), "Demo board profile must be in demo/"
    assert (demo_dir / "app.js").exists()
    assert (demo_dir / "style.css").exists()
    assert (demo_dir / "themes.css").exists()

    # Landing page iframe embed check
    landing_html = (out_site / "index.html").read_text(encoding="utf-8")
    assert 'src="demo/lab.html?embed=1"' in landing_html, "Hero section must embed demo iframe"
    assert "Launch Fullscreen Workbench" in landing_html

    # Embed mode support in webui
    webui_html = (demo_dir / "lab.html").read_text(encoding="utf-8")
    assert "is-embedded" in webui_html


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
