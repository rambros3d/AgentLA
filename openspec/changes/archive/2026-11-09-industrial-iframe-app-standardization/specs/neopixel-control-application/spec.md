## MODIFIED Requirements

### Requirement: 60 FPS Procedural Shader Engine
The application SHALL provide a client-side procedural animation engine capable of generating dynamic visual patterns (Rainbow Cycle, Cylon Eye, Fire Flicker, Meteor Trail, Breathing, Plasma) and streaming real-time frames at up to 60 FPS. All shader animation controls, Blackout, FPS counter, and power telemetry badges SHALL reside inside a dedicated in-app sub-toolbar located beneath `#top-bar`, preserving full control functionality when `#top-bar` is suppressed in embedded iframe mode.

#### Scenario: Running Rainbow animation
- **WHEN** user selects the Rainbow effect and clicks Start
- **THEN** the browser continuously updates LED hues using `requestAnimationFrame` and streams binary frame updates over the WebSocket connection.

#### Scenario: Shader controls accessible in embedded iframe mode
- **WHEN** the Neopixel application is embedded inside the AgentLA OS launcher iframe
- **THEN** the shader triggers, Blackout button, and telemetry indicators remain visible and interactive within the in-app sub-toolbar.

## ADDED Requirements

### Requirement: Neopixel Industrial Styling and Embedded Viewport
The Neopixel application SHALL apply the Industrial Design System (`data-style="industrial"`, `data-theme="nothing"` / `industrial-light`, NDot57 typography, dot-grid canvas, and zero emojis), and SHALL provide at least 72px bottom clearance padding to avoid occlusion by the host system dock when embedded.

#### Scenario: Visualizer canvas dock clearance
- **WHEN** user scrolls through matrix configuration or animation parameters
- **THEN** the main workspace displays with at least 72px bottom clearance above the dock.
