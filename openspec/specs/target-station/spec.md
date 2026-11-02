# target-station Specification

## Purpose
Specifies the dedicated Target Station surface consolidating firmware flashing, chip auto-detection, interactive CMSIS-DAP/SWD debugging, and target power management.

## Requirements

### Requirement: Consolidated Target Station Surface
The WebUI SHALL provide a dedicated Target Station view accessible from the top-level navigation that groups target programming, verification, and live debugging into a unified workspace.

#### Scenario: Switching to Target Station
- **WHEN** the user selects the `[ 🎯 Target Station ]` navigation tab
- **THEN** the active bench view SHALL be hidden and the Target Station view SHALL become active

#### Scenario: Target Power Control
- **WHEN** the user toggles the VTarget power rail switch in Target Station
- **THEN** the system SHALL send the power command to the probe and visually indicate the rail status and measured voltage

### Requirement: Drag-and-Drop Flasher & Verification
The Target Station SHALL allow drag-and-drop firmware flashing with automatic file type detection and real-time progress reporting.

#### Scenario: Firmware binary dropped
- **WHEN** the user drops a `.bin`, `.hex`, or `.elf` file onto the Target Station drop zone
- **THEN** the file details (name, size, checksum) SHALL be parsed and displayed ready for flashing

### Requirement: Interactive Target Debugger
The Target Station SHALL provide direct SWD/CMSIS-DAP inspection controls including target halt, run, step, core register display, and arbitrary memory read/write inspection.

#### Scenario: Halting target CPU
- **WHEN** the user clicks "Halt" in the debugger panel
- **THEN** the probe SHALL halt the attached Cortex-M/RISC-V target core and populate the register table
