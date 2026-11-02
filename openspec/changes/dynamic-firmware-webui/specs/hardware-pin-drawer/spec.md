## MODIFIED Requirements

### Requirement: Full-Screen Hardware Pin Routing Matrix View
The WebUI SHALL provide a dedicated full-screen workspace view (`#view-pin-matrix`) for inspecting and reconfiguring hardware pin assignments dynamically populated from the active board manifest.

#### Scenario: Opening Pin Matrix via navigation or hotkey
- **WHEN** the user clicks the `[ Pin Matrix ]` tab or presses the `4` or `P` key
- **THEN** the WebUI SHALL display the Pin Matrix view dynamically populated from `board.json`

#### Scenario: Pin conflict detection
- **WHEN** two conflicting instruments or bus masters are assigned to the same physical GPIO pin
- **THEN** the Pin Matrix SHALL highlight the conflicting pin with an amber warning badge and show the competing functions

## ADDED Requirements

### Requirement: Dual-Tier Fixed vs Flexible Pin Rendering
The Pin Matrix SHALL partition pins into two distinct architectural sections based on hardware constraints declared in `board.json`:
1. **Fixed Hardware Functions**: Dedicated physical blocks (Programmer Header, Differential Transceivers, Analog Scope, Target Power) grouped with their physical connector pinouts and voltage rail/level-shifter tags (`1.2V - 5.0V`).
2. **Unified Flexible Digital Pool**: Unconstrained digital GPIOs (PIO state machines, digital logic analyzer channels, signal generator outputs, soft bus masters) presented as a unified, assignable pool.

#### Scenario: Rendering BakuProbe 48-GPIO zones
- **WHEN** the connected board is `bakuprobe`
- **THEN** the Pin Matrix SHALL render fixed function groups (Programmer GP0–GP7, Transceivers GP12–GP19, Analog Scope GP40–GP47) with level-shifting badges, alongside the Unified Flexible Digital Pool (GP20–GP39)

#### Scenario: Rendering Pico 2 30-GPIO zones
- **WHEN** the connected board is `pico2`
- **THEN** the Pin Matrix SHALL render Fixed Programmer (GP0–GP7) and Analog Scope (GP26–GP29), alongside the Unified Flexible Digital Pool (GP8–GP25)
