## ADDED Requirements

### Requirement: Board and Joint Identity Display
The BareFOC Studio application SHALL display silicon hardware UID, board name, board node ID, and per-joint addresses and names.

#### Scenario: Displaying board identity
- **WHEN** the application connects to a BareFOC target
- **THEN** it queries `REG_HARDWARE_UID`, `REG_BOARD_NAME`, and `REG_BOARD_NODE_ID` and displays them in the status strip.

#### Scenario: Editing joint identity
- **WHEN** the user selects a motor tab (M0..M3) and updates the joint address or name
- **THEN** the application writes the updated values to `REG_JOINTx_ADDR` and `REG_JOINTx_NAME` over MultiBus.

### Requirement: Enhanced Calibration Wizard and Safe Disarm Status
The BareFOC Studio application SHALL display motor calibration state, electrical zero angle readout, 1-click ALIGN auto-commit, and mechanical zeroing controls.

#### Scenario: Executing ALIGN from Studio
- **WHEN** the user clicks `Run Electrical ALIGN`
- **THEN** the application writes `0x01` to MultiBus register `0xB0`, updates the state indicator during alignment, and verifies auto-commit to Flash.
