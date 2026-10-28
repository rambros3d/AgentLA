## ADDED Requirements

### Requirement: 5-Tier Hardware Verification Suite
The system SHALL provide an automated Python verification test script `test/test_stm32f1_hardware_suite.py` that executes a 5-tier validation process on connected STM32F1 (Blue Pill) hardware targets over serial CDC.

#### Scenario: Successful 5-tier execution
- **WHEN** the user runs `python3 test/test_stm32f1_hardware_suite.py /dev/ttyACM0`
- **THEN** the test suite executes Tiers 1 through 5 in sequence, asserting that transport, core control, memory bus, FPEC flash programming, and GPIO18 signal logic analyzer sampling pass with 0 errors.

### Requirement: Automatic SWD WAIT & Clock Fallback Handling
The system SWD driver SHALL automatically handle `SWD_ACK_WAIT` (`011`) responses by issuing `DP_ABORT` (0x0000001E) and `DP_CTRLSTAT` (0x50000000) requests while temporarily lowering the SWD clock frequency to 500 kHz / 100 kHz.

#### Scenario: Recovery from SWD WAIT state
- **WHEN** the connected STM32F1 target returns `SWD_ACK_WAIT` (`011`) during DPIDR read
- **THEN** the driver issues `DP_ABORT`, scales SWD clock frequency down to 500 kHz, retries up to 25 times, and successfully obtains `DPIDR = 0x1BA01477`.

### Requirement: CDC Command STM32 TEST & SWD DIAG
The system firmware SHALL handle the CDC serial commands `STM32 TEST` and `SWD DIAG` by probing target DPIDR, powering up DP domains via `DP_CTRLSTAT`, reading `AHB-AP IDR`, detecting DBGMCU part ID `0x410`, reading Flash size geometry, and reporting pin logic levels.

#### Scenario: CDC STM32 TEST on connected Blue Pill
- **WHEN** the user sends `STM32 TEST\r\n` over the CDC serial port `/dev/ttyACM0`
- **THEN** the probe returns `OK STM32 TEST PASS idcode=0x1BA01477 target=STM32F103 (Medium Density) flash=64K ram=20K`.

#### Scenario: CDC SWD DIAG pin status query
- **WHEN** the user sends `SWD DIAG\r\n` over CDC serial port `/dev/ttyACM0`
- **THEN** the probe returns `OK SWD DIAG swclk=4 swdio=5 nreset=6 target_ack=0x60000000`.

### Requirement: GPIO18 Target Signal Sampling
The system logic analyzer engine SHALL sample probe `GPIO18` (connected to Blue Pill `PC13`) for 3 seconds to measure high/low state transitions emitted by the target application.

#### Scenario: PC13 LED Blink Signal Validation
- **WHEN** the target executes the PC13 LED blink firmware
- **THEN** the logic analyzer sampling on `GPIO18` detects at least 25 HIGH samples and 25 LOW samples with high/low state transitions over 3 seconds.
