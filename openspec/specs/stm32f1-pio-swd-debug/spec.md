# stm32f1-pio-swd-debug Specification

## Purpose
TBD - created by archiving change stm32f1-pio-swd-debug. Update Purpose after archive.
## Requirements
### Requirement: Pure PIO SWD Protocol Driver
The system SHALL execute SWD protocol transactions (line reset, JTAG-to-SWD select sequence, DPIDR read, DP/AP register read/write) using the RP2350 PIO0 state machine without resorting to software bitbanging fallbacks.

#### Scenario: Read DPIDR via pure PIO
- **WHEN** the user executes `SWD PIO_IDCODE\r\n` over CDC serial
- **THEN** the probe executes `swd_read_idcode()` using pure PIO and returns `OK SWD PIO_IDCODE 0x1BA01477`.

### Requirement: Target Detection & Debug via PIO
The system SHALL perform full target memory and register operations over PIO SWD, successfully identifying the Blue Pill target MCU and running memory reads.

#### Scenario: Execute STM32 TEST over pure PIO
- **WHEN** the user executes `STM32 TEST\r\n` over CDC serial
- **THEN** the probe returns `OK STM32 TEST PASS idcode=0x1BA01477 target=STM32F103`.

