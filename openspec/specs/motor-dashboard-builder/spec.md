# motor-dashboard-builder Specification

## Purpose
User-configurable dashboards built from discovered parameters, modeled on the ODrive GUI dashboard, including named dashboards, JSON import/export of layouts and full target configuration, and a default Tuning workflow.

## Requirements
### Requirement: Configurable Dashboards
The studio SHALL support multiple named dashboards, each containing user-added controls, actions, and plots selected from the parameter tree, with add and remove operations available in the UI.

#### Scenario: Creating a dashboard
- **WHEN** user chooses to add a dashboard
- **THEN** a new named dashboard is created and becomes the active view

#### Scenario: Adding and removing widgets
- **WHEN** user selects a parameter and adds it as a control, slider, function, action, or plot, or removes an existing widget
- **THEN** the dashboard's widget list is updated immediately and reflected in the view

#### Scenario: Switching dashboards
- **WHEN** user selects a dashboard tab
- **THEN** the studio renders that dashboard's widgets and keeps other dashboards intact

### Requirement: Default Tuning Dashboard
On first target connection the studio SHALL create a default Tuning dashboard equivalent to the ODrive GUI tuning workflow: current and velocity limits, position/velocity/integrator gain sliders, a requested-state enum, a save-configuration function, two setpoint actions, and command-versus-estimate plots.

#### Scenario: Auto-created tuning dashboard
- **WHEN** the first target connects
- **THEN** a Tuning dashboard with the standard tuning controls, actions, and plots is added and its plotted variables are registered for sampling

#### Scenario: Tuning workflow available
- **WHEN** user opens the Tuning dashboard
- **THEN** user can set limits, choose closed-loop state, send a setpoint step, observe command-versus-estimate plots, tune gains, and save configuration

### Requirement: Dashboard And Configuration Import Export
The studio SHALL export and import dashboard layouts as JSON and export and import full target configuration as JSON, preserving values and types.

#### Scenario: Export dashboard
- **WHEN** user selects Export dash
- **THEN** studio downloads the current dashboard as a JSON file containing its controls, actions, and plots

#### Scenario: Import dashboard
- **WHEN** user imports a dashboard JSON file
- **THEN** studio registers a new dashboard and adds all plot variables to the sampled set

#### Scenario: Export target configuration
- **WHEN** user selects Export target config for a discovered target
- **THEN** studio downloads a JSON tree of that target's configuration parameters with values and types preserved

#### Scenario: Import target configuration
- **WHEN** user imports a target configuration JSON file
- **THEN** studio applies the contained parameter values to the selected target through its adapter
