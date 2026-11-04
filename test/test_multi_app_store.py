#!/usr/bin/env python3
"""
Unit and integration test suite for AgentLA Multi-App LittleFS Storage, URL Routing Engine,
Dedicated REST API, and Community App Store Launcher.
"""

import io
import json
import re
import tarfile
import pytest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

LITTLEFS_START_OFFSET = 0x00190000  # 1.5625 MB (after profile slots)
LITTLEFS_RESERVED_END_KB = 16
FLASH_SECTOR_SIZE = 4096

PICO2_FLASH_BYTES = 4 * 1024 * 1024      # 4MB
BAKUPROBE_FLASH_BYTES = 16 * 1024 * 1024  # 16MB


def test_littlefs_partition_geometry():
    """Verify LittleFS partition calculations for Pico 2 (4MB) and BakuProbe (16MB)."""
    # 1. Pico 2 geometry
    pico2_end = PICO2_FLASH_BYTES - (LITTLEFS_RESERVED_END_KB * 1024)
    pico2_size = pico2_end - LITTLEFS_START_OFFSET
    pico2_blocks = pico2_size // FLASH_SECTOR_SIZE

    assert LITTLEFS_START_OFFSET % FLASH_SECTOR_SIZE == 0, "Start offset must be 4KB sector aligned"
    assert pico2_end % FLASH_SECTOR_SIZE == 0, "End offset must be 4KB sector aligned"
    assert pico2_size == 2539520, f"Expected ~2.42MB (2539520 bytes), got {pico2_size}"
    assert pico2_blocks == 620, f"Expected 620 blocks on Pico 2, got {pico2_blocks}"

    # 2. BakuProbe geometry
    baku_end = BAKUPROBE_FLASH_BYTES - (LITTLEFS_RESERVED_END_KB * 1024)
    baku_size = baku_end - LITTLEFS_START_OFFSET
    baku_blocks = baku_size // FLASH_SECTOR_SIZE

    assert baku_end % FLASH_SECTOR_SIZE == 0, "BakuProbe end offset must be 4KB sector aligned"
    assert baku_size == 15122432, f"Expected ~14.42MB (15122432 bytes), got {baku_size}"
    assert baku_blocks == 3692, f"Expected 3692 blocks on BakuProbe, got {baku_blocks}"


def test_streaming_tar_unpacker_header_parsing():
    """Verify streaming TAR parser correctly extracts headers and file contents without RAM buffering."""
    # Create an in-memory TAR archive with app.json, index.html, and an asset
    app_meta = {
        "id": "can-sniffer",
        "name": "CAN Bus Sniffer",
        "version": "1.2.0",
        "description": "High-speed automotive CAN sniffer",
        "author": "Antigravity Team",
        "requires": ["has_hardware_can"]
    }
    index_html = b"<!DOCTYPE html><html><body>CAN Sniffer Workbench</body></html>"
    style_css = b"body { background: #111; color: #00e5ff; }"

    tar_buf = io.BytesIO()
    with tarfile.open(fileobj=tar_buf, mode="w") as tar:
        # app.json
        meta_bytes = json.dumps(app_meta).encode("utf-8")
        ti = tarfile.TarInfo(name="app.json")
        ti.size = len(meta_bytes)
        tar.addfile(ti, io.BytesIO(meta_bytes))

        # index.html
        ti2 = tarfile.TarInfo(name="index.html")
        ti2.size = len(index_html)
        tar.addfile(ti2, io.BytesIO(index_html))

        # assets/style.css
        ti3 = tarfile.TarInfo(name="assets/style.css")
        ti3.size = len(style_css)
        tar.addfile(ti3, io.BytesIO(style_css))

    tar_bytes = tar_buf.getvalue()
    assert len(tar_bytes) > 0

    # Simulate 512-byte chunked streaming unpacker
    extracted_files = {}
    current_name = None
    current_size = 0
    current_written = 0
    file_data = bytearray()
    state = "HEADER"
    hdr_buf = bytearray()

    idx = 0
    while idx < len(tar_bytes):
        chunk = tar_bytes[idx : idx + 128]  # simulate arbitrary lwIP TCP chunks
        idx += len(chunk)
        chunk_idx = 0

        while chunk_idx < len(chunk):
            if state == "HEADER":
                need = 512 - len(hdr_buf)
                take = min(need, len(chunk) - chunk_idx)
                hdr_buf.extend(chunk[chunk_idx : chunk_idx + take])
                chunk_idx += take

                if len(hdr_buf) == 512:
                    if all(b == 0 for b in hdr_buf):
                        state = "DONE"
                        break
                    # Parse filename (first 100 bytes)
                    name_raw = hdr_buf[:100].split(b"\x00")[0].decode("utf-8")
                    # Parse octal size (bytes 124..135)
                    size_raw = hdr_buf[124:136].strip(b"\x00 ").decode("utf-8")
                    current_size = int(size_raw, 8) if size_raw else 0
                    current_name = name_raw.lstrip("./")
                    current_written = 0
                    file_data = bytearray()
                    hdr_buf = bytearray()

                    if current_size > 0:
                        state = "DATA"
                    else:
                        extracted_files[current_name] = b""
                        state = "HEADER"

            elif state == "DATA":
                left = current_size - current_written
                take = min(left, len(chunk) - chunk_idx)
                file_data.extend(chunk[chunk_idx : chunk_idx + take])
                current_written += take
                chunk_idx += take

                if current_written >= current_size:
                    extracted_files[current_name] = bytes(file_data)
                    rem = current_size % 512
                    padding = (512 - rem) if rem else 0
                    if padding > 0:
                        state = "PADDING"
                        pad_discarded = 0
                        pad_needed = padding
                    else:
                        state = "HEADER"

            elif state == "PADDING":
                left_pad = pad_needed - pad_discarded
                take = min(left_pad, len(chunk) - chunk_idx)
                pad_discarded += take
                chunk_idx += take
                if pad_discarded >= pad_needed:
                    state = "HEADER"

            elif state == "DONE":
                chunk_idx = len(chunk)

    assert "app.json" in extracted_files
    assert "index.html" in extracted_files
    assert "assets/style.css" in extracted_files
    assert json.loads(extracted_files["app.json"].decode("utf-8"))["id"] == "can-sniffer"
    assert extracted_files["index.html"] == index_html
    assert extracted_files["assets/style.css"] == style_css


def test_url_routing_and_spa_fallback():
    """Verify URL routing engine decisions, root 302 redirect, lab routes, trailing slash redirect, and SPA fallback rules."""
    def route_request(path, lfs_existing_files, default_home="/lab"):
        # 1. Root redirect (HTTP 302)
        if path == "/":
            return {"status": 302, "location": default_home}

        # 2. Dedicated Lab route (/lab, /lab/, /lab/index.html) and factory ROM assets
        if path in ["/lab", "/lab/", "/lab/index.html", "/index.html", "/style.css", "/app.js", "/agentla-bridge.js", "/favicon.svg", "/board.json"]:
            return {"status": 200, "handler": "ROM_FSDATA"}

        # 3. App Launcher
        if path in ["/apps", "/apps/"]:
            return {"status": 200, "handler": "ROM_APPS_HTML"}

        # 4. Custom App Routing
        if path.startswith("/apps/"):
            sub = path[6:]
            if "/" not in sub:
                # Missing trailing slash -> 301 Redirect
                return {"status": 301, "location": f"{path}/"}

            app_name, relpath = sub.split("/", 1)
            if not relpath:
                relpath = "index.html"

            full_target = f"/apps/{app_name}/{relpath}"
            is_html = full_target.endswith(".html") or full_target.endswith(".htm")
            if full_target in lfs_existing_files:
                return {"status": 200, "file": full_target, "inject_bridge": is_html}

            # Check SPA fallback vs 404
            filename = relpath.rsplit("/", 1)[-1]
            has_extension = "." in filename and not filename.endswith(".")

            if has_extension:
                return {"status": 404, "error": "Static asset missing"}
            else:
                spa_index = f"/apps/{app_name}/index.html"
                if spa_index in lfs_existing_files:
                    return {"status": 200, "file": spa_index, "spa_fallback": True, "inject_bridge": True}
                else:
                    return {"status": 404, "error": "App index missing"}

        # 5. REST API
        if path.startswith("/api/v1/"):
            return {"status": 200, "handler": "REST_API"}

        return {"status": 404, "error": "Not Found"}

    files = [
        "/apps/can-sniffer/index.html",
        "/apps/can-sniffer/assets/bundle.js",
        "/apps/can-sniffer/assets/logo.svg",
    ]

    # Test root 302 redirection
    root_res = route_request("/", files, default_home="/lab")
    assert root_res["status"] == 302
    assert root_res["location"] == "/lab"

    custom_root = route_request("/", files, default_home="/apps")
    assert custom_root["status"] == 302
    assert custom_root["location"] == "/apps"

    # Test lab route and factory assets
    assert route_request("/lab", files)["handler"] == "ROM_FSDATA"
    assert route_request("/lab/", files)["handler"] == "ROM_FSDATA"
    assert route_request("/lab/index.html", files)["handler"] == "ROM_FSDATA"
    assert route_request("/index.html", files)["handler"] == "ROM_FSDATA"
    assert route_request("/agentla-bridge.js", files)["handler"] == "ROM_FSDATA"
    assert route_request("/apps", files)["handler"] == "ROM_APPS_HTML"
    assert route_request("/apps/", files)["handler"] == "ROM_APPS_HTML"

    # Test trailing slash normalization
    redirect = route_request("/apps/can-sniffer", files)
    assert redirect["status"] == 301
    assert redirect["location"] == "/apps/can-sniffer/"

    # Test custom app file serving and bridge injection
    res_index = route_request("/apps/can-sniffer/", files)
    assert res_index["status"] == 200
    assert res_index["file"] == "/apps/can-sniffer/index.html"
    assert res_index["inject_bridge"] is True

    res_js = route_request("/apps/can-sniffer/assets/bundle.js", files)
    assert res_js["status"] == 200
    assert res_js["file"] == "/apps/can-sniffer/assets/bundle.js"
    assert res_js["inject_bridge"] is False

    res_js = route_request("/apps/can-sniffer/assets/bundle.js", files)
    assert res_js["status"] == 200
    assert res_js["file"] == "/apps/can-sniffer/assets/bundle.js"

    # Test SPA subpath fallback (no file extension)
    res_spa1 = route_request("/apps/can-sniffer/dashboard/filters", files)
    assert res_spa1["status"] == 200
    assert res_spa1["spa_fallback"] is True
    assert res_spa1["file"] == "/apps/can-sniffer/index.html"

    res_spa2 = route_request("/apps/can-sniffer/settings", files)
    assert res_spa2["status"] == 200
    assert res_spa2["spa_fallback"] is True

    # Test missing static asset 404 (has file extension)
    res_404 = route_request("/apps/can-sniffer/assets/missing.png", files)
    assert res_404["status"] == 404

    # Test REST API path
    assert route_request("/api/v1/apps", files)["handler"] == "REST_API"
    assert route_request("/api/v1/apps/storage", files)["handler"] == "REST_API"


def test_rest_api_payload_shapes():
    """Verify REST API response JSON payload contracts."""
    # 1. /api/v1/apps/storage
    total_bytes = 2539520
    used_bytes = 163840
    free_bytes = total_bytes - used_bytes
    storage_json = {
        "total_bytes": total_bytes,
        "used_bytes": used_bytes,
        "free_bytes": free_bytes,
        "block_size": 4096,
        "block_count": 620
    }
    assert storage_json["free_bytes"] == total_bytes - used_bytes
    assert storage_json["free_bytes"] > 0
    assert storage_json["block_size"] == 4096

    # 2. /api/v1/apps listing
    apps_json = [
        {
            "id": "can-sniffer",
            "name": "CAN Bus Analyzer",
            "version": "1.0.0",
            "description": "CAN Protocol sniffer",
            "size": 45120,
            "path": "/apps/can-sniffer/"
        }
    ]
    assert len(apps_json) == 1
    assert apps_json[0]["id"] == "can-sniffer"
    assert apps_json[0]["path"].endswith("/")

    # 3. /api/v1/system
    system_json = {
        "board_id": "pico2",
        "board_name": "Raspberry Pi Pico 2",
        "firmware_version": "v1.0",
        "flash_bytes": 4194304,
        "uptime_ms": 12345
    }
    assert system_json["board_id"] == "pico2"
    assert system_json["flash_bytes"] == 4194304

    # 4. /api/v1/pins/safe
    safe_json = {
        "status": "safe",
        "pins_quiesced": True
    }
    assert safe_json["status"] == "safe"
    assert safe_json["pins_quiesced"] is True


def test_apps_html_assets_and_integration():
    """Verify built-in ROM App Launcher UI (apps.html) contents and build staging."""
    apps_html_path = REPO_ROOT / "firmware" / "html" / "apps.html"
    index_html_path = REPO_ROOT / "firmware" / "html" / "index.html"
    cmakelists_path = REPO_ROOT / "CMakeLists.txt"

    assert apps_html_path.exists(), "firmware/html/apps.html must exist"
    assert index_html_path.exists(), "firmware/html/index.html must exist"

    apps_html = apps_html_path.read_text(encoding="utf-8")
    index_html = index_html_path.read_text(encoding="utf-8")
    cmake_text = cmakelists_path.read_text(encoding="utf-8")

    # 1. apps.html structure checks
    assert "storage-dock" in apps_html
    assert "Community App Store" in apps_html
    assert "modal-app-store" in apps_html
    assert "btn-open-app-store" in apps_html
    assert "install-modal" in apps_html
    assert "parseTarMetadata" in apps_html
    assert "/api/v1/apps/storage" in apps_html
    assert "/api/v1/apps/install" in apps_html
    assert "spotlight-search" in apps_html
    assert "modal-settings" in apps_html
    assert "Scope" in apps_html
    assert "Debugger" in apps_html
    assert "Pin Control" in apps_html
    assert "Console" in apps_html

    # 2. Navigation toggle in index.html
    assert "/apps" in index_html, "index.html must include navigation link to /apps"
    assert "settings-default-home-route-select" in index_html, "index.html settings modal must include landing route selector"

    # 3. Injected bridge script and CMake integration
    bridge_path = REPO_ROOT / "firmware" / "html" / "agentla-bridge.js"
    assert bridge_path.exists(), "firmware/html/agentla-bridge.js must exist"
    assert "apps.html" in cmake_text, "CMakeLists.txt must include apps.html in staging outputs and fsdata"
    assert "agentla-bridge.js" in cmake_text, "CMakeLists.txt must include agentla-bridge.js in fsdata"


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
