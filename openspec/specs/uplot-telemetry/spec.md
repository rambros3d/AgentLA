# uplot-telemetry Specification

## Purpose
High-performance timeseries charting library integration for real-time WebUI oscilloscopes, telemetry strip charts, and waveform visualization across AgentLA instruments.
## Requirements
### Requirement: Embedded uPlot Charting Distribution
The system SHALL bundle `uplot.min.js` and `uplot.min.css` directly within `webui/` for zero-overhead, offline-capable timeseries plotting across WebUI applications.

#### Scenario: Script loading in application
- **WHEN** a WebUI application loads `uplot.min.js` and `uplot.min.css`
- **THEN** the global `uPlot` constructor is available without external network access or CDN dependencies.

### Requirement: High-Frequency Timeseries Strip Chart
The application SHALL utilize `uPlot` to render multi-channel real-time motor telemetry strip charts with synchronized time axes, pan, zoom, and interactive cursor inspection.

#### Scenario: Real-time telemetry feed
- **WHEN** telemetry samples arrive from the target over WebSocket or HTTP polling
- **THEN** `uPlot.setData()` updates waveform traces within a sliding ring buffer window maintaining up to 60 FPS rendering.

#### Scenario: Interactive inspection and zooming
- **WHEN** the user drags to select a time region or hovers over the plot
- **THEN** `uPlot` displays exact numerical values for each trace at the cursor position and zooms the selected window.

#### Scenario: Trace visibility toggle
- **WHEN** the user clicks a trace name in the legend
- **THEN** `uPlot` toggles the display of the selected channel without dropping buffered data.

