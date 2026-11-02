# probe-pin-configuration Specification

## Purpose
TBD - created by archiving change unified-mso-scope-and-pin-config. Update Purpose after archive.
## Requirements
### Requirement: Interactive Probe Pins Bar
The Scope view SHALL render an interactive pins bar displaying all observation-capable hardware pins (`GP10` through `GP29`) across the top of the waveform window, with real-time visual status badges indicating their configured mode and trace color.

#### Scenario: Pin status badge display
- **WHEN** pins are rendered in the top pins bar
- **THEN** unconfigured pins display a subtle muted outline
- **AND** active digital pins display a circular status badge filled with the channel's assigned trace color
- **AND** active analog pins (`GP26`–`GP29`) display a diamond badge (`◈`) with an `ADC` tag

#### Scenario: Opening pin configuration
- **WHEN** the user clicks any pin button in the top pins bar
- **THEN** the Interactive Pin Configuration Modal (`#modal-pin-config`) opens focused on that pin

### Requirement: Interactive Pin Configuration Modal
The WebUI SHALL provide a modal dialog allowing users to configure pin role, channel label, trace color via an 18-color preset palette, and hardware-accurate trigger rules.

#### Scenario: Restricting analog ADC role to valid pins
- **WHEN** configuring pin `GP10` through `GP25`
- **THEN** the modal offers `Digital Input (Logic Analyzer)` and `Released / High-Z`, disabling the `Analog Input` option with an informational badge indicating ADC is available on `GP26–GP29`
- **WHEN** configuring pin `GP26` through `GP29`
- **THEN** the modal enables both `Digital Input` and `Analog Input (Oscilloscope ADC)` options

#### Scenario: Curated 18 preset trace color selection
- **WHEN** selecting a trace color in the pin configuration modal
- **THEN** the UI presents a 6 × 3 grid of 18 high-contrast preset color swatches without a freeform color wheel or picker
- **AND** clicking a swatch immediately updates the channel preview badge and marks the swatch as selected

#### Scenario: Hardware-accurate digital trigger configuration
- **WHEN** configuring a digital GPIO pin
- **THEN** the trigger options SHALL be strictly logical: `Rising Edge ⤊`, `Falling Edge ⤋`, `Either Edge ⇅`, `High 1`, `Low 0`, or `None`
- **AND** the UI SHALL NOT display any voltage threshold comparator inputs for digital pins

#### Scenario: Hardware-accurate analog trigger configuration
- **WHEN** configuring an analog ADC pin (`GP26`–`GP29`)
- **THEN** the trigger options SHALL include threshold voltage comparators with edge direction (`Rising ⤊`, `Falling ⤋`) and level in volts (`0.0V` to `3.3V`)

