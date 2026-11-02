# unified-active-bench Specification

## Purpose
Specifies the combined observation and stimulus workbench layout, responsive breakpoints, docked stimulus deck, and synchronized "Send & Sniff" transaction capture.

## ADDED Requirements

### Requirement: Responsive Active Bench Layout
The WebUI SHALL display a unified active bench containing both the waveform capture canvas and the interactive stimulus deck in a responsive arrangement.

#### Scenario: Desktop side-by-side display
- **WHEN** the browser viewport width is greater than or equal to 1200px
- **THEN** the active bench SHALL render side-by-side with the waveform canvas occupying the left (~65%) and the stimulus deck docked on the right (~35%)

#### Scenario: Mobile and compact split-deck display
- **WHEN** the browser viewport width is less than 1200px
- **THEN** the active bench SHALL render stacked vertically with the waveform canvas on top and the stimulus deck at the bottom

#### Scenario: Responsive resize observation
- **WHEN** the viewport or container dimensions change
- **THEN** the waveform canvas SHALL automatically adjust its buffer dimensions without losing time pan or zoom position

### Requirement: Docked Stimulus Deck
The active bench SHALL provide a docked tabbed stimulus deck with immediate access to bus masters, stimulus command REPL, pattern generator, and GPIO pin togglers.

#### Scenario: Switching stimulus tools
- **WHEN** the user selects a tab inside the stimulus deck (Bus Masters, REPL, Pattern Gen, or Pins)
- **THEN** the stimulus deck SHALL switch views without navigating away from or hiding the live waveform canvas

### Requirement: Synchronized Send and Sniff
The system SHALL support an integrated "Send & Sniff" workflow where transmitting bus master commands automatically triggers signal observation.

#### Scenario: Auto-sniff armed on bus transmission
- **WHEN** the user transmits an I2C, SPI, UART, CAN, Modbus, 1-Wire, or LIN transaction and `Auto-Sniff` is active
- **THEN** the system SHALL immediately arm logic analyzer capture if idle, receive the transaction waveform, and center the timeline on the transmitted packet

#### Scenario: Auto-sniff toggling
- **WHEN** the user toggles the `Auto-Sniff on Transmit` toolbar button
- **THEN** the system SHALL update the auto-sniff preference and persist it in local storage
