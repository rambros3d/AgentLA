## MODIFIED Requirements

### Requirement: Top-Rail Navigation Integration
The system SHALL display the `NeoPixel` instrument in the top navigation rail across all primary system workbench pages.

#### Scenario: Navigating from other system instruments
- **WHEN** user is viewing `/scope`, `/terminal`, `/barefoc`, `/foc`, or `/servo`
- **THEN** the top navigation rail displays a link to `NeoPixel` (`/neopixel`)
