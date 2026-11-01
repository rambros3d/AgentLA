## Why

MIPI I3C is the modern successor to I2C and SPI, offering speeds up to 12.5 MHz SDR and HDR-DDR with In-Band Interrupts (IBI), Dynamic Address Assignment (ENTDAA), and standardized Common Command Codes (CCC). While AgentLA supports I2C, SPI, UART, and CAN in its Adapter domain, it lacks I3C master capabilities and I3C protocol testing.

Adding I3C controller support to AgentLA empowers embedded engineers to interactively scan, provision, control, and test modern I3C targets (such as sensors, PMICs, secure elements, and DDR5 SPDs) directly over USB CLI and Web UI. Furthermore, validating this implementation against real silicon—specifically the STMicroelectronics STM32H503 (Cortex-M33 with native MIPI I3C v1.1 target peripheral)—ensures specification compliance, real-world timing integrity, and robust target interop.

## What Changes

- **I3C Controller Engine**: Integrate a hardware-accelerated I3C Controller into AgentLA's Adapter domain, adapted from the reference `I3CBlaster` PIO architecture.
- **Dedicated PIO2 Allocation**: Dedicate RP2350 `PIO2` to the I3C engine, preventing any instruction memory or state machine collisions with SWD/CMSIS-DAP (`PIO0`) or Logic Analyzer DMA capture (`PIO1`).
- **Dynamic Clock Rate Scaling**: Compute PIO clock divisors dynamically using `clock_get_hz(clk_sys)` to guarantee accurate SCL clocking (from 100 kHz up to 12.5 MHz) regardless of system frequency (150 MHz, 200 MHz, or 400 MHz Turbo).
- **Adapter Pin & Ownership Integration**: Map I3C SCL and SDA to the Adapter pin domain (`GPIO 10` and `GPIO 11`) via `pin_owner` claim/release mechanics.
- **CLI & Web Command Bridge**: Expose interactive I3C commands (`I3C_SCAN`, `I3C_ENTDAA`, `I3C_RSTDAA`, `I3C_READ`, `I3C_WRITE`, `I3C_CCC`, `I3C_POLL`, `I3C_RESET`).
- **STM32H503 Verification Suite**: Implement an end-to-end hardware-in-the-loop test suite that programs an STM32H503 target via SWD with an I3C Target firmware fixture, performs bus discovery and DAA, verifies private SDR transfers, tests CCC exchanges, and checks In-Band Interrupt (IBI) detection.

## Capabilities

### New Capabilities
- `i3c-adapter`: Hardware-accelerated MIPI I3C master controller implementing open-drain and push-pull SDR signaling, Dynamic Address Assignment (ENTDAA), Common Command Codes (CCC), private read/write, and In-Band Interrupt (IBI) polling on Adapter pins.
- `i3c-verification-stm32h503`: Hardware-in-the-loop verification harness combining SWD target flashing, test firmware provisioning on the STM32H503 I3C peripheral, and automated bus validation test cases.

### Modified Capabilities
<!-- None: existing I2C, SWD, and Logic Analyzer specs remain unaltered -->

## Impact

- **Firmware**: New modules in `firmware/src/adapter/` (`adapter_i3c.c`, `adapter_i3c.h`, `firmware/pio/i3c.pio`), command handling in `firmware/src/main.c`, and build rules in `CMakeLists.txt`.
- **Hardware Resources**: Claims `PIO2` on RP2350 and claims pins in the Adapter domain (`GPIO 10` SCL, `GPIO 11` SDA).
- **Target Microcontroller**: Leverages existing STM32H503 SWD flashing algorithms in AgentLA to provision target firmware for validation.
- **Host Tools**: New test automation scripts under `test/` verifying I3C operations.
