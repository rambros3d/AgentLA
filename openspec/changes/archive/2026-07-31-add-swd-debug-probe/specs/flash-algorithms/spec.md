## ADDED Requirements

### Requirement: Flash Algorithm Interface
The system SHALL provide a pluggable flash algorithm interface for different MCU families.

#### Scenario: Algorithm registration
- **WHEN** firmware initializes
- **THEN** all supported flash algorithms are registered in algorithm table

#### Scenario: Algorithm selection
- **WHEN** target IDCODE matches a registered algorithm
- **THEN** system selects the appropriate flash algorithm

### Requirement: STM32F1 Flash Algorithm
The system SHALL support STM32F1 family flash programming.

#### Scenario: Erase STM32F1 sector
- **WHEN** flash_erase() is called with STM32F1 algorithm
- **THEN** specified sector is erased (1KB per sector)

#### Scenario: Write STM32F1 flash
- **WHEN** flash_write() is called with STM32F1 algorithm
- **THEN** data is written to flash at specified address

#### Scenario: Read STM32F1 flash
- **WHEN** flash_read() is called with STM32F1 algorithm
- **THEN** data is read from flash at specified address

### Requirement: STM32F4 Flash Algorithm
The system SHALL support STM32F4 family flash programming.

#### Scenario: Erase STM32F4 sector
- **WHEN** flash_erase() is called with STM32F4 algorithm
- **THEN** specified sector is erased (variable sector size)

#### Scenario: Write STM32F4 flash
- **WHEN** flash_write() is called with STM32F4 algorithm
- **THEN** data is written using double-word programming

### Requirement: STM32G0 Flash Algorithm
The system SHALL support STM32G0 family flash programming.

#### Scenario: Erase STM32G0 page
- **WHEN** flash_erase() is called with STM32G0 algorithm
- **THEN** specified page is erased (2KB per page)

#### Scenario: Write STM32G0 flash
- **WHEN** flash_write() is called with STM32G0 algorithm
- **THEN** data is written using half-page programming

### Requirement: RP2040/RP2350 Flash Algorithm
The system SHALL support RP2040/RP2350 flash programming.

#### Scenario: Erase RP2040 flash
- **WHEN** flash_erase() is called with RP2040 algorithm
- **THEN** specified 4KB sector is erased

#### Scenario: Write RP2040 flash
- **WHEN** flash_write() is called with RP2040 algorithm
- **THEN** data is written using XIP flash controller
