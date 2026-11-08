## ADDED Requirements

### Requirement: Industrial Chamfered Rail Dock
The floating bottom dock SHALL render as an Industrial Chamfered Rail with sharp 0.5rem squircle geometry (`border-radius: 8px`), smoked obsidian glass backdrop (`rgba(10, 10, 10, 0.90)`), 1px structural hairline outline (`rgba(255, 255, 255, 0.12)`), vertical dividers between groups, and Nothing Signal Red (`#D71921`) active indicator pips.

#### Scenario: Dock silhouette rendering
- **WHEN** the floating dock renders on any page
- **THEN** it displays an 8px corner radius with 1px hairline border, eliminating rounded 24px pill curves.

#### Scenario: Active tool indicator
- **WHEN** an instrument or page is active
- **THEN** a circular Signal Red pip (`#D71921`) renders centered beneath the active dock button.

#### Scenario: Pinned user apps grouping
- **WHEN** user LittleFS applications are pinned to the dock
- **THEN** they render within a dedicated section bounded by 1px vertical hairline dividers.
