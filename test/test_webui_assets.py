#!/usr/bin/env python3
"""
Unit tests for AgentLA WebUI Assets, Themes, Routing Invariants,
and Floating Action Button Bridge.
"""

from pathlib import Path
import re
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
HTML_DIR = REPO_ROOT / "firmware" / "html"


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


def test_index_html_clean_header_and_landing_setting():
    """Verify index.html has clean top bar (no redundant launcher pills) and landing page selector."""
    index_html_path = HTML_DIR / "index.html"
    assert index_html_path.exists()
    content = index_html_path.read_text(encoding="utf-8")

    # 1. Redundant nav pill removed
    assert '<nav class="header-nav-links"' not in content, "Redundant nav pill container must be removed"
    assert 'class="preset-pill">App Launcher</a>' not in content

    # 2. Brand logo acts as link to /apps
    assert 'href="/apps"' in content, "Brand logo must link to /apps"
    assert 'id="brand-logo"' in content

    # 3. Settings modal includes landing page selector
    assert 'id="settings-default-home-route-select"' in content, "Landing page selector must be present"
    assert 'value="/lab"' in content
    assert 'value="/apps"' in content

    # 4. Uses relative asset links compatible with both firmware and GitHub Pages
    assert 'href="style.css"' in content
    assert 'href="themes.css"' in content
    assert 'src="app.js"' in content


def test_apps_html_unified_os_launcher():
    """Verify apps.html has DaisyUI styling, squircle grid, spotlight search, and shared settings modal."""
    apps_html_path = HTML_DIR / "apps.html"
    assert apps_html_path.exists()
    content = apps_html_path.read_text(encoding="utf-8")

    # 1. No legacy #0d1117 block
    assert "background-color: var(--bg-canvas" in content
    assert 'href="style.css"' in content
    assert 'href="themes.css"' in content

    # 2. Flagship tile for AgentLA Lab
    assert "AgentLA Lab" in content
    assert 'onclick="openLab()"' in content
    assert "FLAGSHIP" in content

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
    """Verify agentla-bridge.js implements closed shadow DOM, win-rar icon, and suppression."""
    bridge_path = HTML_DIR / "agentla-bridge.js"
    assert bridge_path.exists()
    content = bridge_path.read_text(encoding="utf-8")

    assert "attachShadow({ mode: 'closed' })" in content, "Must use closed Shadow DOM"
    assert "no_launcher" in content, "Must support ?no_launcher=1 suppression"
    assert "AgentLANoLauncher" in content, "Must support window.AgentLANoLauncher suppression"
    assert "/apps" in content, "1-tap click must navigate to /apps"
    assert "44px" in content, "Must be 44x44px FAB"
    assert "backdrop-filter" in content, "Must have backdrop-filter blur"


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


def test_index_html_daisyui_markup_components():
    """Verify index.html adopts DaisyUI navbar, telemetry stats, table zebra, and mockup-code."""
    index_html_path = HTML_DIR / "index.html"
    assert index_html_path.exists()
    content = index_html_path.read_text(encoding="utf-8")

    assert "navbar" in content, "index.html top bar must use DaisyUI .navbar"
    assert "navbar-start" in content, "index.html navbar must include .navbar-start"
    assert "navbar-end" in content, "index.html navbar must include .navbar-end"
    assert "scope-telemetry-stats" in content, "index.html scope telemetry container missing"
    assert "stats-horizontal" in content, "index.html scope telemetry must use DaisyUI .stats-horizontal"
    assert "stat-title" in content, "index.html scope telemetry must use .stat-title"
    assert "stat-value" in content, "index.html scope telemetry must use .stat-value"
    assert "table-zebra" in content, "index.html packet/log table must use DaisyUI .table-zebra"
    assert "mockup-code" in content, "index.html terminal display must use DaisyUI .mockup-code"


def test_app_js_dynamic_canvas_token_binding():
    """Verify app.js implements dynamic CSS token caching and theme change listener for canvases."""
    app_js_path = HTML_DIR / "app.js"
    assert app_js_path.exists()
    content = app_js_path.read_text(encoding="utf-8")

    assert "getCanvasPalette" in content, "app.js must provide getCanvasPalette() helper"
    assert "invalidateCanvasPalette" in content, "app.js must provide invalidateCanvasPalette() helper"
    assert "agentla-theme-changed" in content, "app.js must listen for and dispatch agentla-theme-changed"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
