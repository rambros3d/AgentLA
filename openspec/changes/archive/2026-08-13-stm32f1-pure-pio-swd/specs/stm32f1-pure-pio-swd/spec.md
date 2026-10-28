# Delta Spec: Pure PIO SWD Communication

## Added Requirements

### Requirement: Pure PIO SWD DPIDR Read
The PIO SWD engine SHALL execute DPIDR reads over PIO0 at 100 kHz frequency and return `ack = 001 (OK)` and `idcode = 0x1BA01477` on STM32F103 target without relying on bitbanging fallbacks.

#### Scenario: Pure PIO IDCODE Command
- **WHEN** user issues `SWD PIO_IDCODE` over CDC serial `/dev/ttyACM0`
- **THEN** response SHALL be `OK SWD PIO_IDCODE 0x1BA01477`
