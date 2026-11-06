# dynamic-board-manifest Specification

## MODIFIED Requirements

### Requirement: Declarative Board Manifest Definition
The system SHALL provide a structured JSON specification (`board.json` and `/api/board/manifest`) generated from the active board crate defining MCU attributes, peripheral capabilities, electrical rails, level translation, and physical zone pin allocations.

#### Scenario: Valid board manifest schema
- **WHEN** `/api/board/manifest` is requested by build tools, MCP agents, or the WebUI client
- **THEN** it SHALL contain `board_id`, `name`, `version`, `assets`, `mcu`, `features`, and `zones` fields reflecting the active board crate configuration.
