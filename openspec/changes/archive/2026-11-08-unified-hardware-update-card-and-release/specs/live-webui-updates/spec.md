## ADDED Requirements

### Requirement: Browser-Orchestrated 2-Phase Full System Update
The client console SHALL orchestrate a 2-phase full system update when the user triggers `AUTO UPDATE ALL`. Phase 1 SHALL stream `webui.tar` to `POST /api/v1/system/webui/update`, and upon successful extraction, Phase 2 SHALL stream `agentla-pico2-ota.bin` to `POST /api/v1/system/firmware/ota`, followed by an automated reboot prompt to activate the updated firmware.

#### Scenario: Executing full system 2-phase update
- **WHEN** user clicks `[ AUTO UPDATE ALL ]` in the System Update Hero Card
- **THEN** the client downloads `webui.tar` and posts to `/api/v1/system/webui/update`, updates progress to 50%, then downloads `agentla-pico2-ota.bin` and posts to `/api/v1/system/firmware/ota`, updates progress to 100%, and prompts for warm reboot.

#### Scenario: Phase 1 failure prevents firmware staging
- **WHEN** downloading or streaming `webui.tar` encounters a network or flash error
- **THEN** Phase 2 does not execute, leaving the running firmware slot and standby slot untouched.
