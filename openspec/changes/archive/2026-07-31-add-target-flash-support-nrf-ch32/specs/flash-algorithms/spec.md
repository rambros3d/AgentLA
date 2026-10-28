## ADDED Requirements

### Requirement: nRF52840 Flash Algorithm
The system SHALL support Nordic Semi nRF52840 family flash programming via NVMC over SWD.

#### Scenario: Erase nRF52840 page
- **WHEN** flash_erase() is called with nRF52840 algorithm
- **THEN** specified 4KB page is erased via NVMC ERASEPAGE register and NVMC_READY is verified

#### Scenario: Mass erase nRF52840 flash
- **WHEN** flash_erase() is called with mass erase parameter for nRF52840
- **THEN** entire flash array is erased via NVMC ERASEALL register

#### Scenario: Write nRF52840 flash
- **WHEN** flash_write() is called with nRF52840 algorithm
- **THEN** data is written as 32-bit words via NVMC write mode and verified against NVMC_READY

### Requirement: CH32 Flash Algorithm
The system SHALL support WCH CH32 (CH32F1 and CH32V003) family flash programming.

#### Scenario: Erase CH32 page
- **WHEN** flash_erase() is called with CH32 algorithm
- **THEN** specified page is erased via CH32 fast-erase controller sequence

#### Scenario: Write CH32 fast block
- **WHEN** flash_write() is called with CH32 algorithm
- **THEN** data is written in 128-byte fast blocks with bus-stall delay handling
