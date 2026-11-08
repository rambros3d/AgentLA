# hardware-pin-drawer Specification

## Purpose
Specifies the slide-out Hardware Pin Drawer for live GPIO assignment, bus multiplexing, voltage level shifting, and pin conflict detection.

## ADDED Requirements

### Requirement: Slide-out Pin Matrix Drawer
The WebUI SHALL provide a slide-out drawer for inspecting and reconfiguring hardware pin assignments without navigating away from the active workbench.

#### Scenario: Opening drawer via button
- **WHEN** the user clicks the `[ Pins ]` toolbar button
- **THEN** the pin drawer SHALL slide out smoothly from the right side of the screen

#### Scenario: Opening drawer via hotkey
- **WHEN** the user presses the `P` key (while not typing in an input field)
- **THEN** the pin drawer SHALL toggle its open/closed state

#### Scenario: Pin conflict detection
- **WHEN** two conflicting instruments or bus masters are assigned to the same physical GPIO pin
- **THEN** the drawer SHALL highlight the conflicting pin with an amber warning badge and show the competing functions
