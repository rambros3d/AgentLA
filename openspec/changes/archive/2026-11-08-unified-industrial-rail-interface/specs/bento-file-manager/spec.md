## MODIFIED Requirements

### Requirement: Standalone Bento File Manager Page
The system SHALL provide a dedicated standalone WebUI page at `files.html` (accessible via `/files` and `/files.html`) styled using the unified Industrial design language that renders an Industrial Storage Console for both `system_fs` and `user_fs`.

#### Scenario: Navigating to files.html
- **WHEN** a user navigates to `/files` or `/files.html`
- **THEN** the browser loads `files.html` displaying the Industrial Storage Console with dot-matrix section headers, technical monospace breadcrumbs, partition metrics, and directory canvas.

#### Scenario: Theme synchronization
- **WHEN** `files.html` loads
- **THEN** it reads `localStorage` for `agentla-theme` and `agentla-theme-mode` and applies the theme attributes before DOM render without visual flicker.
