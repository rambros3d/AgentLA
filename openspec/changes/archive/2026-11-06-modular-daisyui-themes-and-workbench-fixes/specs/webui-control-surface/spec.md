## ADDED Requirements

### Requirement: Real-Time Adapter Protocol Terminal Logging
The WebUI Adapter view SHALL route all outgoing bus transmissions (UART, I2C, SPI, CAN, Modbus) directly into the protocol's dedicated terminal log window via `appendAdapterLog()`, displaying timestamped direction indicators, hex bytes, and ASCII interpretations.

#### Scenario: Transmitting I2C or SPI write
- **WHEN** the user transmits an I2C or SPI frame from the Adapter tab
- **THEN** the corresponding `#i2c-log-content` or `#spi-log-content` terminal immediately appends a formatted entry displaying `[TX]` timestamp and data bytes
- **AND** the terminal view automatically scrolls to the newest log line.

#### Scenario: Auto-sniff capture notification
- **WHEN** bus transmission initiates with auto-sniff enabled
- **THEN** the adapter terminal displays an informational entry noting capture arming and channel assignment.

### Requirement: Dynamic Desktop Sidebar Navigation Tabs
The desktop vertical navigation sidebar (`.desktop-vertical-nav`) SHALL provide dynamic icon/text tabs matching the mobile bottom navigation pattern, where inactive tabs display only their icon and the active tab dynamically displays both icon and text label.

#### Scenario: Inactive desktop sidebar tab presentation
- **WHEN** a navigation mode button in the desktop sidebar is not active
- **THEN** it displays its icon centered in a compact button footprint with its label collapsed (`opacity: 0`).

#### Scenario: Active desktop sidebar tab presentation
- **WHEN** a navigation mode button becomes active
- **THEN** it dynamically expands to reveal both icon and text label with smooth transition and theme active highlight.

### Requirement: Pin Matrix Instant Action Button Styling
The Pin Matrix view SHALL style instant GPIO control buttons (`.btn-pin-act` for High `H`, Low `L`, Pull-Up `PU`, Pull-Down `PD`, and High-Z `Z`) using compact DaisyUI button group aesthetics with clear active/pressed states.

#### Scenario: Visual presentation of pin action buttons
- **WHEN** the Pin Matrix table renders GPIO pin rows
- **THEN** each row displays a compact button group containing `H`, `L`, `PU`, `PD`, and `Z` buttons styled with theme tokens (`--color-base-200`, `--color-primary`, `--radius-field`)
- **AND** the active pin mode button displays a distinct active accent highlight.

### Requirement: Unified Semantic Color Tokens
The WebUI stylesheets SHALL utilize semantic DaisyUI theme tokens (`var(--color-primary)`, `var(--color-base-100)`, `var(--color-base-content)`, etc.) and purge deprecated hardcoded color tokens (`var(--accent-cyan)`).

#### Scenario: Dynamic theme adaptability of workbench elements
- **WHEN** switching themes
- **THEN** borders, focus rings, headers, tab active highlights, and indicators adapt seamlessly to the new theme's primary and base colors without residual cyan tints.
