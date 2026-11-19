## Context

`webui/barefoc.html` in `AgentLA-rust` connects to target motor controllers over I2C via AgentLA's bus daemon (`/api/v1/bus/i2c/transfer`). It requires updates to support the extended MultiBus register map, multi-motor switching, and interactive kinematics tuning.

## Goals / Non-Goals

**Goals:**
- Provide multi-motor tab switching via `MotorSelect` (`0x70`).
- Provide dedicated kinematics control surface for `0x80..0xBF`.
- Interactive canvas for LUT curve editing and single-burst streaming.
- Compliance with AgentLA zero-emoji and dock geometry standards.

**Non-Goals:**
- Offline trajectory planning (performed in kinematics firmware).

## Decisions

### Decision 1: Canvas Curve Visualization
- **Decision**: Use HTML5 canvas to render piecewise linear LUT profiles and monotonicity curves directly in the browser.
- **Rationale**: Provides instant visual feedback to operators before committing curves to target actuator hardware.

### Decision 2: Zero Emoji & Surface Active Styling
- **Decision**: Adhere strictly to AGENTS.md rules: no emoji characters, no CSS dot indicators, using subtle background highlights for active tabs.
