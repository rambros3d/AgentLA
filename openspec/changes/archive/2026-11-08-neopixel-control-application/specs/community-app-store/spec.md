## ADDED Requirements

### Requirement: Neopixel Control App Catalog Registration
The official app catalog `apps/catalog.json` and packaging pipeline SHALL register the `neopixel-control` application with required metadata, tags, and dependencies.

#### Scenario: Discovering Neopixel Control in App Store
- **WHEN** user views the App Store catalog
- **THEN** `neopixel-control` appears with title "Neopixel Control", category "Lighting", icon `icon.svg`, and a link to install or launch
