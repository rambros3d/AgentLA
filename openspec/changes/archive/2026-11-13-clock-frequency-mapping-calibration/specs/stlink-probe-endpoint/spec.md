## ADDED Requirements

### Requirement: Calibrated ST-Link SWD Frequency Configuration
The system SHALL handle command `DBG_APIV2_SWD_SET_FREQ` (`0xF2 0x43`) by mapping the 16-bit divisor parameter through the calibrated frequency mapping table and reconfiguring the hardware SWD clock divider.

#### Scenario: Set SWD clock frequency via ST-Link divisor command
- **WHEN** host sends `0xF2 0x43` with divisor 0
- **THEN** system reconfigures the SWD transport clock to 4000 kHz and returns status OK
