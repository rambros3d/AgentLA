## MODIFIED Requirements

### Requirement: Web Terminal Application UI and Hardware Reset
The firmware SHALL serve an in-browser serial terminal application at `/apps/terminal/index.html` (and `/terminal.html`) with baud rate selection, pin configuration, local echo toggle, log download, and a 1-click hardware target reset button, styled with the Industrial Design System (`data-style="industrial"`, `data-theme="nothing"`). The UI layout SHALL be inverted such that the command input bar (`#cmd-input` and `#btn-send`) and configuration controls reside in a sticky top-anchored sub-toolbar directly beneath the connection header, incoming log lines are prepended in reverse-chronological order (newest lines at the top immediately below the input bar), and the scrollable terminal display container maintains at least 72px bottom clearance padding to avoid occlusion by the floating system dock.

#### Scenario: One-Click Target Cold Boot
- **WHEN** user clicks the "Reset Target" button in the terminal UI
- **THEN** the firmware asserts a 50 ms active-low pulse on `nRESET` (GPIO 6), rebooting the target MCU while keeping the terminal session open to capture the startup output.

#### Scenario: Top-Anchored Command Input and Reverse Stream
- **WHEN** incoming target serial text arrives or a user sends a command via `#cmd-input`
- **THEN** the newest text output appears immediately below the top input bar without requiring the user to scroll to the bottom of the container.

#### Scenario: Embedded Dock Clearance
- **WHEN** the serial terminal runs inside an embedded iframe with `html.is-embedded`
- **THEN** the main terminal output viewport reserves 72px of bottom clearance above the dock, ensuring all terminal history remains fully viewable.
