## MODIFIED Requirements

### Requirement: Standalone Bento File Manager Page
The system SHALL provide a dedicated standalone WebUI page at `files.html` (accessible via `/files` and `/files.html`) styled using the unified Industrial design language that renders an Industrial Storage Console for both `system_fs` and `user_fs`. The top header element SHALL bear `id="top-bar"` so it is cleanly suppressed in embedded iframe mode (`html.is-embedded #top-bar { display: none !important; }`), the partition selector (`user:/` vs `sys:/`) SHALL be integrated directly into the explorer sub-toolbar beneath the top header, and the file listing canvas SHALL maintain at least 72px bottom clearance padding to avoid dock occlusion.

#### Scenario: Navigating to files.html
- **WHEN** a user navigates to `/files` or `/files.html`
- **THEN** the browser loads `files.html` displaying the Industrial Storage Console with dot-matrix section headers, technical monospace breadcrumbs, partition metrics, and directory canvas.

#### Scenario: Theme synchronization
- **WHEN** `files.html` loads
- **THEN** it reads `localStorage` for `agentla-theme` and `agentla-theme-mode` and applies the theme attributes before DOM render without visual flicker.

#### Scenario: Embedded iframe top bar suppression
- **WHEN** `files.html` is embedded inside the AgentLA OS launcher iframe
- **THEN** `#top-bar` is hidden without duplicate navigation headers, and the partition switcher remains functional within the in-app explorer sub-toolbar.
