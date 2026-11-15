## ADDED Requirements

### Requirement: Static Storage of Agent Skills in LittleFS
The probe SHALL store the 4 agent skill definition files (`agentla-dev`, `agentla-debug`, `agentla-programmer`, `agentla-analyzer`) as static markdown files under `/skills/<name>/SKILL.md` within the WebUI LittleFS partition.

#### Scenario: Serve skill over HTTP
- **WHEN** an HTTP client requests `GET /skills/agentla-dev/SKILL.md`
- **THEN** the probe HTTP server returns `200 OK` with `Content-Type: text/markdown; charset=utf-8` and the full skill contents.

### Requirement: Interactive WebUI Skills Console
The WebUI SHALL display an interactive Agent Skills viewer allowing users and agents to browse available skills, inspect requirements, and copy system prompts.

#### Scenario: Copy agent prompt from WebUI
- **WHEN** an engineer views the Skills console in the WebUI and clicks "Copy Prompt" for a skill
- **THEN** the browser copies the skill prompt instructions into the clipboard with notification feedback.

### Requirement: LittleFS Partition Headroom Guard
Packaging the skills suite SHALL NOT cause the LittleFS compressed image to exceed the 512 KB static partition allocation.

#### Scenario: Build and verify partition size
- **WHEN** `pack_webui.py` builds the LittleFS binary including the skills suite
- **THEN** the generated binary size is verified to be under 524,288 bytes (with at least 200 KB free headroom).
