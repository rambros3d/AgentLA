#!/usr/bin/env python3
"""
Unit and integration tests for Servo Master example app and high-resolution PWM.
Tests microsecond math, REST endpoints, app manifest, and starter app seeding.
"""

import json
from pathlib import Path
import pytest


def test_microsecond_pwm_calculations():
    f_sys = 150_000_000  # 150 MHz RP2350
    freq = 50  # 50 Hz frame rate (20 ms period)

    # 1. Clock divider for 1 MHz tick resolution (1 us per tick)
    div = f_sys / 1_000_000.0
    assert div == 150.0, f"Expected divider 150.0, got {div}"

    # 2. Wrap value for 20,000 us period
    wrap = int(1_000_000 / freq) - 1
    assert wrap == 19999, f"Expected wrap 19999, got {wrap}"

    # 3. Compare levels for standard servo positions
    min_us = 500
    max_us = 2500
    center_us = 1500

    def angle_to_us(angle: int) -> int:
        return int(min_us + (angle * (max_us - min_us)) / 180)

    assert angle_to_us(0) == 500
    assert angle_to_us(90) == 1500
    assert angle_to_us(180) == 2500

    # Continuous rotation throttle to us
    def throttle_to_us(throttle: int) -> int:
        if -5 <= throttle <= 5:
            return 1500
        if throttle < -5:
            return int(min_us + (throttle + 100) * (1480 - min_us) / 95)
        return int(1520 + (throttle - 5) * (max_us - 1520) / 95)

    assert throttle_to_us(0) == 1500  # Deadband
    assert throttle_to_us(-100) == 500  # Full reverse
    assert throttle_to_us(100) == 2500  # Full forward


def test_servo_master_manifest_schema():
    manifest_file = Path("apps/servo-master/app.json")
    assert manifest_file.exists(), "apps/servo-master/app.json must exist"

    manifest = json.loads(manifest_file.read_text(encoding="utf-8"))
    assert manifest["name"] == "servo-master"
    assert "Servo Master" in manifest["title"]
    assert "1.0.0" in manifest["version"]
    assert "author" in manifest
    assert "description" in manifest
    assert manifest["icon"].startswith("data:image/svg+xml")
    assert "pwm" in manifest.get("requires", [])
    assert manifest.get("min_firmware_version") == "1.0.0"


def test_servo_master_html_capabilities():
    html_file = Path("apps/servo-master/index.html")
    assert html_file.exists(), "apps/servo-master/index.html must exist"

    content = html_file.read_text(encoding="utf-8")

    # 1. Responsive layout and dark theme
    assert 'data-theme="dark"' in content
    assert "Servo Master" in content

    # 2. Rotating SVG horn and continuous rotation gear
    assert "servo-dial-svg" in content
    assert "horn-group" in content
    assert "gear-group" in content

    # 3. Motion easing & Auto-sweep
    assert "chk-easing" in content
    assert "btn-sweep-toggle" in content

    # 4. Keyframe Pose Sequencer
    assert "btn-save-pose" in content
    assert "btn-play-seq" in content
    assert "sequence-timeline" in content

    # 5. HTML5 Gamepad API
    assert "navigator.getGamepads" in content
    assert "gamepad-badge" in content

    # 6. Interactive wiring guide & safety modal
    assert "modal-wiring" in content
    assert "External 5V / 6V Power Supply" in content
    assert "USB brownout resets" in content

    # 7. Dedicated REST API calls
    assert "/api/v1/servo/attach" in content
    assert "/api/v1/servo/write" in content
    assert "/api/v1/servo/detach" in content


def test_seed_apps_implementation():
    seed_h = Path("firmware/src/seed_apps.h")
    seed_c = Path("firmware/src/seed_apps.c")
    assert seed_h.exists(), "firmware/src/seed_apps.h must exist"
    assert seed_c.exists(), "firmware/src/seed_apps.c must exist"

    c_content = seed_c.read_text(encoding="utf-8")
    assert "servo_master_app_json" in c_content
    assert "servo_master_index_html" in c_content
    assert "seed_default_apps" in c_content
    assert "/apps/servo-master/app.json" in c_content
    assert "/apps/servo-master/index.html" in c_content


def test_rest_api_servo_pwm_routes():
    rest_api_c = Path("firmware/src/rest_api.c")
    assert rest_api_c.exists(), "firmware/src/rest_api.c must exist"

    content = rest_api_c.read_text(encoding="utf-8")

    # PWM routes
    assert 'strncmp(path, "pwm", 3) == 0' in content
    assert 'strncmp(sub, "claim", 5) == 0' in content
    assert 'strncmp(sub, "set", 3) == 0' in content
    assert 'strncmp(sub, "release", 7) == 0' in content

    # Servo routes
    assert 'strncmp(path, "servo", 5) == 0' in content
    assert 'strncmp(sub, "attach", 6) == 0' in content
    assert 'strncmp(sub, "write_us", 8) == 0' in content
    assert 'strncmp(sub, "write", 5) == 0' in content
    assert 'strncmp(sub, "detach", 6) == 0' in content
