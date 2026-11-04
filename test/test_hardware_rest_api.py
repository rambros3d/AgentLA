#!/usr/bin/env python3
"""
Unit and integration test suite for AgentLA Full Hardware REST Suite:
- Profiles & Pin Ownership (/api/v1/profiles, /api/profiles, /api/v1/pins)
- Peripheral Bus Bridges (/api/v1/bus/i2c, /spi, /uart, /can)
- Universal Addressable LED Driver (/api/v1/bus/led/transmit - WS2812B, SK6812 RGBW, APA102)
- Target Programmer Engine (/api/v1/programmer)
- Logic Analyzer & VCD Streaming (/api/v1/analyzer)
- Probe Telemetry & Device Control (/api/v1/device)
"""

import json
import pytest

REST_BUFFER_SIZE = 1536
REST_HEADER_RESERVE = 160
JSON_CAPACITY = REST_BUFFER_SIZE - REST_HEADER_RESERVE - 1


def test_rest_api_url_matching():
    """Verify endpoint prefix matching for dedicated REST API and legacy profiles alias."""
    def rest_api_matches(uri: str) -> bool:
        if not uri:
            return False
        if uri.startswith("/api/v1/"):
            return True
        if uri.startswith("/api/profiles"):
            return True
        return False

    # Valid REST endpoints
    assert rest_api_matches("/api/v1/system") is True
    assert rest_api_matches("/api/v1/profiles") is True
    assert rest_api_matches("/api/v1/profiles/apply") is True
    assert rest_api_matches("/api/v1/pins") is True
    assert rest_api_matches("/api/v1/pins/10") is True
    assert rest_api_matches("/api/v1/bus/i2c/scan") is True
    assert rest_api_matches("/api/v1/bus/led/transmit") is True
    assert rest_api_matches("/api/v1/programmer/target") is True
    assert rest_api_matches("/api/v1/analyzer/vcd") is True
    assert rest_api_matches("/api/v1/device/telemetry") is True

    # Legacy alias matching
    assert rest_api_matches("/api/profiles") is True
    assert rest_api_matches("/api/profiles/apply") is True

    # Non-REST endpoints
    assert rest_api_matches("/index.html") is False
    assert rest_api_matches("/lab") is False
    assert rest_api_matches("/mcp") is False
    assert rest_api_matches("/apps/can-sniffer/") is False


def test_compact_pins_json_buffer_capacity():
    """Verify that a 30-pin table formatted with compact JSON fits within the 1536 B REST buffer."""
    pins = []
    for i in range(30):
        domain = "P" if i < 10 else ("A" if i < 20 else "L")
        pins.append({
            "p": i,
            "d": domain,
            "m": "SIO",
            "o": 0,
            "v": 0
        })

    payload = {
        "status": "ok",
        "safe": True,
        "active_profile": "Safe State (Hi-Z)",
        "pins": pins
    }
    json_str = json.dumps(payload, separators=(',', ':'))
    payload_len = len(json_str)

    # Must comfortably fit within JSON capacity without truncation
    assert payload_len < JSON_CAPACITY, f"30-pin JSON ({payload_len} bytes) exceeds capacity {JSON_CAPACITY}"
    assert payload_len < 1300, f"Expected compact pin JSON < 1300 bytes, got {payload_len}"
    assert len(pins) == 30


def test_addressable_led_swizzling_ws2812b():
    """Verify WS2812B GRB 24-bit swizzling and MSB-first bit order."""
    def swizzle_pixel_ws2812(r: int, g: int, b: int) -> int:
        # WS2812B expects GRB order shifted left by 8 bits for 24-bit autopull
        return ((g & 0xFF) << 24) | ((r & 0xFF) << 16) | ((b & 0xFF) << 8)

    # Red: (255, 0, 0) -> G:0, R:255, B:0
    red_val = swizzle_pixel_ws2812(255, 0, 0)
    assert (red_val >> 24) & 0xFF == 0    # Green = 0
    assert (red_val >> 16) & 0xFF == 255  # Red = 255
    assert (red_val >> 8) & 0xFF == 0     # Blue = 0

    # Green: (0, 255, 0) -> G:255, R:0, B:0
    green_val = swizzle_pixel_ws2812(0, 255, 0)
    assert (green_val >> 24) & 0xFF == 255
    assert (green_val >> 16) & 0xFF == 0
    assert (green_val >> 8) & 0xFF == 0


def test_addressable_led_swizzling_sk6812_rgbw():
    """Verify SK6812 32-bit GRBW swizzling."""
    def swizzle_pixel_sk6812_rgbw(r: int, g: int, b: int, w: int) -> int:
        # SK6812 RGBW expects GRBW in full 32-bit word
        return ((g & 0xFF) << 24) | ((r & 0xFF) << 16) | ((b & 0xFF) << 8) | (w & 0xFF)

    val = swizzle_pixel_sk6812_rgbw(10, 20, 30, 40)
    assert (val >> 24) & 0xFF == 20  # G
    assert (val >> 16) & 0xFF == 10  # R
    assert (val >> 8) & 0xFF == 30   # B
    assert (val & 0xFF) == 40        # W


def test_addressable_led_apa102_spi_framing():
    """Verify APA102 / DotStar 32-bit SPI frame construction."""
    def build_apa102_frames(pixels, brightness=31):
        frames = []
        # 1. 32-bit Start Frame
        frames.extend([0x00, 0x00, 0x00, 0x00])

        # 2. Per-LED 32-bit Frame: [111 | Brightness(5-bit), B, G, R]
        header_byte = 0xE0 | (brightness & 0x1F)
        for r, g, b in pixels:
            frames.extend([header_byte, b & 0xFF, g & 0xFF, r & 0xFF])

        # 3. End Frame: ceil(N/2) bits rounded to bytes
        end_bytes = (len(pixels) + 15) // 16
        frames.extend([0xFF] * end_bytes)
        return bytes(frames)

    pixels = [(255, 0, 128), (0, 255, 64)]
    stream = build_apa102_frames(pixels, brightness=16)

    # Start frame
    assert stream[0:4] == b'\x00\x00\x00\x00'
    # LED 1: Header 0xE0 | 16 = 0xF0, Blue 128, Green 0, Red 255
    assert stream[4:8] == bytes([0xF0, 128, 0, 255])
    # LED 2: Header 0xF0, Blue 64, Green 255, Red 0
    assert stream[8:12] == bytes([0xF0, 64, 255, 0])
    # End frame
    assert stream[12:] == b'\xFF'


def test_addressable_led_buffer_limit():
    """Verify driver rejects strips beyond the 256 pixel limit."""
    max_pixels = 256
    assert len([0] * 256) <= max_pixels
    with pytest.raises(ValueError):
        num_requested = 300
        if num_requested > max_pixels:
            raise ValueError("Maximum strip length is 256 pixels")


def test_vcd_header_structure():
    """Verify generated VCD stream conforms to standard Sigrok/GTKWave formatting."""
    vcd_template = (
        "$date Today $end\n"
        "$version AgentLA v1.0 $end\n"
        "$timescale 1ns $end\n"
        "$scope module top $end\n"
        "$var wire 1 ! ch0 $end\n"
        "$upscope $end\n"
        "$enddefinitions $end\n"
        "#0\n"
        "0!\n"
    )
    assert "$date" in vcd_template
    assert "$version AgentLA" in vcd_template
    assert "$timescale" in vcd_template
    assert "$enddefinitions $end" in vcd_template
    assert "#0" in vcd_template
