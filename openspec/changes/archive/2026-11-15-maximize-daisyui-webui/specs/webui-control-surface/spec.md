## MODIFIED Requirements

### Requirement: Console Tab Agent Guide Modal
The Console view SHALL provide an "Agent Guide" button (`#btn-agent-guide`) in its header actions that opens an interactive modal dialog implemented via native HTML5 `<dialog id="modal-agent-guide" class="modal">` and `.modal-box`, presenting unified MCP setup instructions, transport endpoints, tools overview, and sample prompts for autonomous AI coding agents (Antigravity, Claude, OpenCode, Cline, Cursor).

#### Scenario: User opens Agent Guide
- **WHEN** user clicks `#btn-agent-guide` in the Console header
- **THEN** `#modal-agent-guide` opens via `showModal()` displaying universal MCP configuration instructions and a copyable JSON snippet inside `.modal-box`

#### Scenario: User copies universal MCP configuration
- **WHEN** user clicks `#btn-copy-mcp-config` inside the Agent Guide modal
- **THEN** the universal `mcpServers.agentla` JSON snippet is copied to clipboard and the button text temporarily confirms "Copied!"

#### Scenario: User dismisses Agent Guide modal
- **WHEN** user presses Escape or clicks the close button in `.modal-action`
- **THEN** `#modal-agent-guide` closes cleanly via native dialog dismissal
