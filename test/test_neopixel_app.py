#!/usr/bin/env python3
"""
Unit and integration tests for Neopixel Control app and LED WebSocket streaming bridge.
Tests manifest schema, UI capabilities, shader engines, firmware routes, and catalog discovery.
"""

import json
import tarfile
from pathlib import Path
import pytest


def test_neopixel_manifest_schema():
    manifest_file = Path("apps/neopixel-control/app.json")
    assert manifest_file.exists(), "apps/neopixel-control/app.json must exist"

    manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
    assert manifest["name"] == "neopixel-control"
    assert "Neopixel Control" in manifest["title"]
    assert "1.0.0" in manifest["version"]
    assert manifest["author"] == "AgentLA Team"
    assert manifest["category"] == "Lighting"
    assert "Lighting" in manifest.get("tags", [])
    assert "ws2812" in manifest.get("requires", [])
    assert "gpio_flexible" in manifest.get("requires", [])
    assert manifest["icon"].endswith(".svg") or manifest["icon"].startswith("data:image/svg+xml")

    icon_file = Path("apps/neopixel-control/icon.svg")
    assert icon_file.exists(), "apps/neopixel-control/icon.svg must exist"
    icon_content = icon_file.read_text(encoding="utf-8")
    assert "<svg" in icon_content and "</svg>" in icon_content


def test_neopixel_html_capabilities():
    html_file = Path("apps/neopixel-control/index.html")
    assert html_file.exists(), "apps/neopixel-control/index.html must exist"

    content = html_file.read_text(encoding="utf-8")

    # 1. DaisyUI v5 styling and Bento design
    assert "/daisyui.css" in content
    assert "/style.css" in content
    assert "/themes.css" in content
    assert "Neopixel Control" in content

    # 2. Multi-protocol support (1-wire and 2-wire SPI)
    for proto in ["ws2812b", "sk6812", "ws2811", "ws2813", "ws2815", "apa102", "sk9822", "ws2801"]:
        assert proto in content, f"Expected protocol '{proto}' in index.html"

    # 3. Color Order mapping
    for order in ["GRB", "RGB", "BGR", "RGBW", "GRBW"]:
        assert order in content, f"Expected color order '{order}' in index.html"

    # 4. Geometry options (1D Strip, 1D Ring, 2D Matrix with Serpentine)
    assert "geom-strip" in content
    assert "geom-ring" in content
    assert "geom-matrix" in content
    assert "chk-serpentine" in content
    assert "chk-col-major" in content

    # 5. Interactive Visualizer and Painting Tools
    assert "led-canvas" in content
    assert "tool-brush" in content
    assert "tool-fill" in content
    assert "tool-eraser" in content

    # 6. Color picker and dedicated SK6812 White Channel
    assert "picker-native" in content
    assert "slider-w-channel" in content

    # 7. 60 FPS Procedural Shaders
    for shader in ["rainbow", "cylon", "fire", "meteor", "pulse", "plasma"]:
        assert shader in content, f"Expected shader '{shader}' in index.html"

    # 8. Real-time Power Telemetry & USB Safety Badge
    assert "telemetry-ma" in content
    assert "power-badge" in content
    assert "power-progress" in content

    # 9. Dedicated WebSocket Endpoint
    assert "/api/v1/led/ws" in content


def test_firmware_led_ws_endpoint():
    web_rs = Path("firmware-rs/src/web.rs")
    assert web_rs.exists(), "firmware-rs/src/web.rs must exist"
    web_code = web_rs.read_text(encoding="utf-8")
    assert "/api/v1/led/ws" in web_code

    rest_rs = Path("firmware-rs/src/rest.rs")
    assert rest_rs.exists(), "firmware-rs/src/rest.rs must exist"
    rest_code = rest_rs.read_text(encoding="utf-8")
    assert "LedWebSocket" in rest_code
    assert "LedWsCallback" in rest_code
    assert "transmit_raw_bytes_pin" in rest_code
    assert "transmit_clocked_bytes" in rest_code

    ws2812_rs = Path("firmware-rs/src/bridge/ws2812.rs")
    assert ws2812_rs.exists(), "firmware-rs/src/bridge/ws2812.rs must exist"
    ws2812_code = ws2812_rs.read_text(encoding="utf-8")
    assert "transmit_raw_bytes_pin" in ws2812_code
    assert "transmit_clocked_bytes" in ws2812_code


def test_catalog_discovery_and_package():
    catalog_file = Path("apps/catalog.json")
    assert catalog_file.exists(), "apps/catalog.json must exist"

    catalog = json.loads(catalog_file.read_text(encoding="utf-8"))
    app_ids = [a["id"] for a in catalog.get("apps", [])]
    assert "neopixel-control" in app_ids, "neopixel-control must be registered in apps/catalog.json"

    entry = next(a for a in catalog["apps"] if a["id"] == "neopixel-control")
    assert entry["name"] == "Neopixel Control"
    assert entry["category"] == "Lighting"
    assert entry["package_url"] == "/packages/neopixel-control.tar"
    assert entry["size_bytes"] > 0

    tar_path = Path("apps/packages/neopixel-control.tar")
    assert tar_path.exists(), "apps/packages/neopixel-control.tar must exist"

    with tarfile.open(tar_path, "r") as tar:
        names = tar.getnames()
        assert "app.json" in names
        assert "icon.svg" in names
        assert "index.html" in names
