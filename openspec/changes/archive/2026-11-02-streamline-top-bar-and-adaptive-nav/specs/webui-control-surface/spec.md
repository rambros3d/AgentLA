## MODIFIED Requirements

### Requirement: WebUI lifecycle initialization & Start menu control
The WebUI JavaScript SHALL execute without scoping errors on page load, attach the `DOMContentLoaded` listener, fade out the loading overlay, and manage the five primary top-level modes (`Active Bench`, `Adapters`, `Debugger`, `Pin Matrix`, and `Console`) via the left vertical sidebar on desktop and bottom navigation on mobile, without rendering deprecated demo connection buttons.

#### Scenario: Successful WebUI boot
- **WHEN** the browser navigates to the WebUI root URL
- **THEN** `#loading-screen` receives the `fade-out` class within 500ms and `Active Bench` is selected as the default active mode without rendering `Connect USB` or `← Portal` buttons in the top header

#### Scenario: Switching primary modes via sidebar or mobile nav
- **WHEN** the user clicks any mode button (`#nav-mode-bench`, `#nav-mode-adapters`, `#nav-mode-target`, `#nav-mode-pins`, or `#nav-mode-console`)
- **THEN** the active tab button receives the `.active` styling and the corresponding top-level view container is displayed while hiding inactive views

## ADDED Requirements

### Requirement: Open Source Project Link in Header
The WebUI SHALL display an Open Source Initiative (OSI) keyhole icon button immediately to the left of the theme toggle in the top application bar, linking to the project documentation site.

#### Scenario: User navigates to Open Source site
- **WHEN** user clicks the OSI icon button (`#btn-open-source`) in the header
- **THEN** the browser opens `https://rambros3d.github.io/AgentLA` in a new browser tab (`target="_blank" rel="noopener noreferrer"`) without interrupting active bench measurements or captures

### Requirement: Console Tab Agent Guide Modal
The Console view SHALL provide an "Agent Guide" button (`#btn-agent-guide`) in its header actions that opens an interactive modal dialog presenting unified MCP setup instructions, transport endpoints, tools overview, and sample prompts for autonomous AI coding agents (Antigravity, Claude, OpenCode, Cline, Cursor).

#### Scenario: User opens Agent Guide
- **WHEN** user clicks `#btn-agent-guide` in the Console header
- **THEN** `#modal-agent-guide` opens displaying universal MCP configuration instructions and a copyable JSON snippet

#### Scenario: User copies universal MCP configuration
- **WHEN** user clicks `#btn-copy-mcp-config` inside the Agent Guide modal
- **THEN** the universal `mcpServers.agentla` JSON snippet is copied to clipboard and the button text temporarily confirms "Copied!"

### Requirement: Console Text Wrapping & Viewport Containment
The Console terminal window SHALL wrap long unbreakable words, hex dumps, and serialized JSON without forcing the container or page to overflow horizontally beyond the viewport.

#### Scenario: Rendering long unbreakable hex or JSON response
- **WHEN** the console receives a response string longer than the terminal container width without spaces
- **THEN** the text breaks onto the next line within `.panel.terminal-display` via `overflow-wrap: anywhere; word-break: break-word;` and `.console-main-grid` maintains strict viewport boundaries

### Requirement: Console Semantic Theming and Click-to-Copy
The Console output display SHALL format command inputs, standard echoes, success results, and errors using semantic theme tokens, and SHALL copy individual lines to the system clipboard upon click.

#### Scenario: Differentiating command input from output
- **WHEN** a user or agent submits a command
- **THEN** the prompt and command text renders in `var(--color-primary)` and responses render in `var(--color-base-content)` (or `var(--color-success)` / `var(--color-error)`)

#### Scenario: Clicking an output line to copy
- **WHEN** a user clicks on any output line in `#cmd-output-content` without performing a drag-selection
- **THEN** the line text is copied to the clipboard and a brief inline "Copied!" badge is displayed

### Requirement: Terminal Output Clear Badge Button
The Console terminal window (`.panel.terminal-display`) SHALL provide an integrated floating clear badge button (`#btn-cmd-clear`) in its bottom-right corner that clears displayed log output upon click.

#### Scenario: User clears console output
- **WHEN** user clicks `#btn-cmd-clear` located in the bottom-right corner of the terminal window
- **THEN** `#cmd-output-content` text is cleared and the terminal view resets to blank
