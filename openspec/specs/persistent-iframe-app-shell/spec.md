# persistent-iframe-app-shell Specification

## Purpose
TBD - created by archiving change universal-app-shell-and-mcu-optimization. Update Purpose after archive.
## Requirements
### Requirement: Persistent App-Shell Viewport Container
The web desktop shell (`webui/index.html`) SHALL provide a persistent application viewport container (`#view-app-viewport`) holding an isolated application iframe (`#app-viewport-frame`) bounded between `top: 48px` (OS top bar) and `bottom: 84px` (floating glass dock clearance), ensuring embedded applications receive a full-bleed viewport that never overlaps or gets obscured by OS controls.

#### Scenario: Viewport geometry isolation
- **WHEN** an application or instrument is active in the App-Shell
- **THEN** `#view-app-viewport` is visible with `height: calc(100vh - 132px)`, and its embedded iframe cannot collide with `#floating-glass-dock` or `#top-bar`.

### Requirement: Client-Side URL Hash Routing
The App-Shell SHALL listen for browser hash changes and manage navigation state via URL hash routes (`#/`, `#/scope`, `#/terminal`, `#/files`, `#/app/:id`), seamlessly toggling between the Bento Launchpad view and the active application viewport without performing full-page reloads.

#### Scenario: Launch app via URL hash
- **WHEN** the browser navigates to `#/app/servo-master` or the user clicks the Servo Master tile
- **THEN** the App-Shell mounts `/app/servo-master/` into the viewport iframe, switches to the viewport view, and marks the corresponding dock icon active without reloading `index.html`.

#### Scenario: Return to Launcher via logo or hash
- **WHEN** the user clicks the AgentLA brand logo or navigates to `#/`
- **THEN** the App-Shell hides the viewport container and displays `#view-launchpad` while preserving live telemetry connections.

### Requirement: Microcontroller Immutable Static Asset Caching
The firmware HTTP server SHALL serve static WebUI assets (`.css`, `.js`, `.woff2`, `.svg`, `.json`) from `SYSTEM_FS` with `Cache-Control: public, max-age=604800, immutable` headers, enabling client browsers to cache core bundles permanently across sessions and eliminating flash read churn on the RP2350 microcontroller.

#### Scenario: Browser requests static stylesheet
- **WHEN** the browser requests `/style.css` or `/daisyui.css`
- **THEN** the HTTP response contains `Cache-Control: public, max-age=604800, immutable` and `Content-Encoding: gzip` when supported by the client.

