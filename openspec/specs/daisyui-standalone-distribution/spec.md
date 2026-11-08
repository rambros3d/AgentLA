# daisyui-standalone-distribution Specification

## Purpose
System WebUI distribution of pre-compressed standalone daisyUI CSS bundle directly from microcontroller flash with HTTP gzip streaming, long-term browser cache control, and partition headroom limits.

## Requirements

### Requirement: Full Standalone daisyUI 5 CSS Bundle Serving
The AgentLA firmware web server SHALL serve the complete, standalone daisyUI CSS stylesheet at `/daisyui.css` directly from the system partition with pre-compressed gzip streaming and long-term browser cache control.

#### Scenario: Requesting daisyUI stylesheet
- **WHEN** an HTTP client requests `GET /daisyui.css`
- **THEN** the server returns status `200 OK`, `Content-Type: text/css`, `Content-Encoding: gzip`, and `Cache-Control: public, max-age=31536000`

### Requirement: Pre-Gzip Packaging and System Flash Headroom Limit
The WebUI packaging tool `tools/pack_webui.py` SHALL pre-compress `daisyui.css` with gzip and ensure that the total compressed WebUI partition image fits within the 512 KB LittleFS system partition boundary with at least 35% free flash headroom.

#### Scenario: Packaging WebUI partition image
- **WHEN** `pack_webui.py` executes against `webui/` containing `daisyui.css`
- **THEN** it generates a valid LittleFS binary image `webui.img` of exactly 524,288 bytes and reports total gzipped asset size below 340,787 bytes (>= 35% headroom)
