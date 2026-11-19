# motor-arbitrary-waveform-plotting Specification

## Purpose
Oscilloscope plotting in which any discovered numeric parameter can be selected as a trace, while retaining the existing canvas scope capabilities (timebase, autoscale, freeze, crosshair, CSV export, step-response analysis).

## Requirements
### Requirement: Arbitrary Variable Trace Selection
The oscilloscope SHALL allow each trace to be bound to any discoverable numeric parameter, and SHALL allow users to add parameters to the global sampled set from the parameter tree.

#### Scenario: Selecting a trace variable
- **WHEN** user chooses a trace source
- **THEN** the list of choices includes all numeric parameters registered for sampling, not only a fixed set of telemetry fields

#### Scenario: Sampling a discovered parameter
- **WHEN** user adds a discovered numeric parameter to a plot
- **THEN** studio registers that parameter for sampling and it becomes available as a trace source

#### Scenario: Global sampling control
- **WHEN** user clicks start sampling or stop sampling in the header
- **THEN** studio begins or ends streaming sampled values for all registered parameters and plots update accordingly

### Requirement: Retained Scope Capabilities
The oscilloscope SHALL retain the existing canvas ring-buffer rendering, selectable timebases, autoscale, freeze/resume, interactive crosshair with value label, FPS display, CSV export, and step-response transient analyzer.

#### Scenario: Interactive crosshair
- **WHEN** user moves the pointer over the scope canvas
- **THEN** studio displays a vertical crosshair and a label with the time and trace values at the cursor position

#### Scenario: Freeze and resume
- **WHEN** user clicks Freeze
- **THEN** the displayed waveform holds its current samples and clicking Resume restarts live rendering

#### Scenario: CSV export
- **WHEN** user clicks Export CSV
- **THEN** studio downloads a CSV containing time and all visible trace values

#### Scenario: Step-response analysis
- **WHEN** user runs a step test
- **THEN** studio injects the configured step, records the response, and reports rise time, settling time, overshoot, and steady-state error
