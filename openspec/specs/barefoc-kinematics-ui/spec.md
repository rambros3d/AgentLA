# barefoc-kinematics-ui Specification

## Purpose
TBD - created by archiving change add-barefoc-multibus-kinematics-ui. Update Purpose after archive.
## Requirements
### Requirement: Multi-Motor Tab Switching

`webui/barefoc.html` SHALL provide a multi-motor selector allowing operators to switch between motors 0 through $N-1$ by writing the chosen index to `MotorSelect` (0x70) over I2C.

#### Scenario: Switching to motor 1
- **WHEN** user selects Motor 1 from the tab bar
- **THEN** an I2C write of `[0x70, 0x01]` is dispatched and subsequent telemetry polling reads motor 1.

### Requirement: Kinematics Configuration & Telemetry Panel

`webui/barefoc.html` SHALL display interactive controls for kinematics parameters (gear ratio, backlash, compliance, 2D joint angles, outer PIDs, and differential coupling matrix).

#### Scenario: Updating backlash parameter
- **WHEN** user adjusts the Backlash Delta slider to 0.05 rad
- **THEN** a float write to register 0x82 is dispatched and confirmed.

#### Scenario: Synchronous 2D joint angle monitoring
- **WHEN** telemetry is streaming
- **THEN** dual joint gauges display `JointAngleAlpha` (0x89) and `JointAngleBeta` (0x8A) in real time.

### Requirement: LUT Curve Interactive Upload Tool

`webui/barefoc.html` SHALL provide a curve editor to visualize and stream $N$ sample points to registers 0xA0–0xA4 in a single sequential write burst.

#### Scenario: Burst LUT upload
- **WHEN** user clicks \"Upload Curve\"
- **THEN** a single I2C burst transaction `[0xA3, 0x00, 0x00, val0, val1, ...]` is executed and `LutStatus` (0xA5) is read to confirm monotonicity.

### Requirement: On-Device Calibration & Flash Storage Controls

`webui/barefoc.html` SHALL provide buttons to trigger autonomous stall sweeps, interactive homing, and atomic Save to Flash / Load from Flash commands via register 0xB0.

#### Scenario: Saving calibration to flash
- **WHEN** user clicks \"Save Calibration to Flash\"
- **THEN** command 0x03 is written to register 0xB0 and `FlashStatus` (0xB4) is checked for CRC32 validity.

