# motor-parameter-explorer Specification

## Purpose
Transport-aware discovery of a live typed parameter/function tree from motor targets and rendering of generic typed controls, so any exposed parameter is controllable without editing HTML.

## Requirements
### Requirement: Transport-Aware Parameter Tree Discovery
The studio SHALL discover a typed parameter and function tree from each connected target and normalize it to nodes carrying value, data type, and read-only state.

#### Scenario: Fibre endpoint schema discovery
- **WHEN** a Fibre target connects
- **THEN** studio performs a real endpoint-0 schema read, decodes the object tree to typed nodes, caches it per target, and displays discovered parameters and functions rather than only a hard-coded list

#### Scenario: MultiBus register map discovery
- **WHEN** a MultiBus target connects
- **THEN** studio expands the known register descriptor map (target, enable, control mode, angle, velocity, current, PID gains, voltage/velocity limits) into a typed parameter tree and populates live values over I2C or SPI

#### Scenario: Commander capability discovery
- **WHEN** a Commander target connects
- **THEN** studio probes supported commands and synthesizes a conservative typed tree that distinguishes writable commands from read-only telemetry

### Requirement: Generic Typed Controls From Parameters
The studio SHALL render generic controls from parameter nodes: boolean as checkbox, numeric as evaluating input with unit display, enum as a select, writable numeric as a slider, functions as invokable buttons, and read-only nodes as value displays.

#### Scenario: Control type follows node type
- **WHEN** a boolean, numeric, or enum parameter is added to a dashboard
- **THEN** the appropriate boolean, numeric/input, or select control is rendered automatically from the node metadata

#### Scenario: Slider range derivation
- **WHEN** a writable numeric parameter is rendered as a slider
- **THEN** its initial minimum and maximum are derived from the current value (one quarter and four times the value) and are user-adjustable

#### Scenario: Read-only nodes are not writable
- **WHEN** a parameter node is marked read-only
- **THEN** studio renders its value only and provides no write control for it

#### Scenario: Function invocation
- **WHEN** user invokes a discovered function control
- **THEN** studio calls the corresponding adapter function on the target and reports success or failure in the console

### Requirement: Parameter Tree Browsing
The studio SHALL provide a browsable parameter tree from which users can select nodes to add to dashboards, to plots, or to invoke as actions.

#### Scenario: Browsing discovered nodes
- **WHEN** user opens the parameter tree
- **THEN** nodes are grouped by target and axis and show their current values and types

#### Scenario: Refresh tree
- **WHEN** user requests a tree refresh
- **THEN** studio re-reads discovered nodes and updates values without removing user-created dashboard widgets
