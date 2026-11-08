## ADDED Requirements

### Requirement: Interface Persona Selection in System Settings
The System Settings dialog (`#modal-settings`) SHALL provide an Interface Persona selector allowing the user to switch between `Industrial (Default)` and `Bento`.

#### Scenario: Switching persona to Bento
- **WHEN** user selects "Bento" in System Settings
- **THEN** `data-style` attribute on `<html>` switches to `bento`, `localStorage.setItem('agentla-ui-style', 'bento')` is updated, and the layout updates immediately without page refresh.

#### Scenario: Switching persona to Industrial
- **WHEN** user selects "Industrial" in System Settings
- **THEN** `data-style` attribute on `<html>` switches to `industrial`, `localStorage.setItem('agentla-ui-style', 'industrial')` is updated, and the layout updates immediately without page refresh.

#### Scenario: Instant style hydration without FOUC
- **WHEN** any page (`index.html`, `lab.html`, `terminal.html`, `files.html`) is loaded
- **THEN** an inline synchronous `<script>` in `<head>` queries `localStorage.getItem('agentla-ui-style') || 'industrial'` and sets `document.documentElement.setAttribute('data-style', ...)` before DOM rendering starts.
