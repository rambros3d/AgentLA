## MODIFIED Requirements

### Requirement: Marketing and Product Hero Presentation
The hosted landing page SHALL display an interactive, responsive hero section using pure DaisyUI `.hero`, `.hero-content`, and `.card` components showcasing AgentLA as an RP2350 high-speed multi-tool bench engine with live feature summaries, an embedded interactive simulated device frame rendering live waveforms, pinout details, and action prompts matching the on-device firmware styling.

#### Scenario: Viewing the hero landing page
- **WHEN** a user visits the hosted portal root (`/`)
- **THEN** the landing page renders a DaisyUI hero banner with an interactive `<iframe src="demo/?embed=1">` device preview bezel, feature cards with 1px `base-300` borders, and primary action buttons for "Launch Fullscreen Demo" and "Connect to Device" using DaisyUI `.btn .btn-primary`

## ADDED Requirements

### Requirement: Compact Embedded Demo Mode
The WebUI workbench (`webui/index.html`) SHALL support an `?embed=1` query parameter that hides top-level chrome and brand banners to maximize canvas waveform visibility when presented inside the landing page preview bezel.

#### Scenario: Rendering WebUI in embedded mode
- **WHEN** `webui/index.html?embed=1` is loaded inside an iframe
- **THEN** top application bar brand items and launcher buttons are hidden, and the canvas fills the frame with live simulated signals
