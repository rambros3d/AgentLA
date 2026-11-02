## MODIFIED Requirements

### Requirement: SWD AP Register Write
The SWD engine SHALL write AP registers with automatic bank selection via DP_SELECT and DP_RDBUFF pipeline flush.

#### Scenario: Write AP register in new bank
- **WHEN** swd_write_ap() is called with a register address in a different bank than the previous access
- **THEN** system writes DP_SELECT with the bank bits, reads DP_RDBUFF to flush pipeline, then performs the AP write

#### Scenario: Write AP register in same bank
- **WHEN** swd_write_ap() is called with a register address in the same bank as the previous access
- **THEN** system performs the AP write directly without re-selecting the bank

### Requirement: SWD AP Register Read
The SWD engine SHALL read AP registers with automatic bank selection via DP_SELECT and DP_RDBUFF pipeline flush.

#### Scenario: Read AP register in new bank
- **WHEN** swd_read_ap() is called with a register address in a different bank than the previous access
- **THEN** system writes DP_SELECT with the bank bits, reads DP_RDBUFF to flush pipeline, then performs the AP read

#### Scenario: Read AP register in same bank
- **WHEN** swd_read_ap() is called with a register address in the same bank as the previous access
- **THEN** system performs the AP read directly without re-selecting the bank
