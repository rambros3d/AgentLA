# stm32f1-heartbeat-debug Specification

## Purpose
TBD - created by archiving change stm32f1-heartbeat-debug. Update Purpose after archive.

## Requirements
### Requirement: Heartbeat Double-Pulse Blink Firmware
The system SHALL build a Blue Pill (STM32F103) heartbeat blink firmware via PlatformIO from `test/bluepill_blink/src/main.cpp` that toggles the active-LOW PC13 LED in a double-pulse cardiac rhythm: 100 ms ON / 100 ms OFF / 100 ms ON / 700 ms OFF, repeating on a 1 Hz cycle.

#### Scenario: Heartbeat rhythm output
- **WHEN** the firmware runs on an STM32F103 Blue Pill after reset
- **THEN** the PC13 LED outputs two 100 ms LOW pulses separated by 100 ms HIGH, followed by a 700 ms HIGH rest period, repeating each 1000 ms.

#### Scenario: PlatformIO build produces flashable binary
- **WHEN** the user runs `pio run --project-dir test/bluepill_blink`
- **THEN** a binary is produced at `test/bluepill_blink/.pio/build/bluepill_f103c8/firmware.bin` suitable for SWD flash programming at `0x08000000`.

### Requirement: CDC DEBUG Subcommands for Cortex-M3 Debug
The AgentLA firmware SHALL expose CDC serial commands under the `DEBUG` prefix (`DEBUG HALT`, `DEBUG RUN`, `DEBUG STEP`, `DEBUG REGS`, `DEBUG READ <addr>`, `DEBUG WRITE <addr> <val>`) that drive the Cortex-M3 core debug registers `DHCSR` (`0xE000EDF0`), `DCRSR` (`0xE000EDF4`), and `DCRDR` (`0xE000EDF8`) over SWD.

#### Scenario: Halt the running target
- **WHEN** the user sends `DEBUG HALT\r\n` over the CDC serial port
- **THEN** the probe writes `C_DEBUGEN | C_HALT = 0xA05F0003` to `DHCSR` and responds `OK DEBUG HALT` with the CPU halted.

#### Scenario: Resume the halted target
- **WHEN** the user sends `DEBUG RUN\r\n` over the CDC serial port while the target is halted
- **THEN** the probe writes `0xA05F0000` to `DHCSR` and responds `OK DEBUG RUN` with the CPU resuming execution.

#### Scenario: Single-step the halted target
- **WHEN** the user sends `DEBUG STEP\r\n` over the CDC serial port while the target is halted
- **THEN** the probe sets `C_MASKINTS` and `C_STEP` in `DHCSR` to execute one instruction without entering ISRs and responds `OK DEBUG STEP`.

#### Scenario: Read core registers
- **WHEN** the user sends `DEBUG REGS\r\n` over the CDC serial port while the target is halted
- **THEN** the probe reads R0-R15 and xPSR via `DCRSR`/`DCRDR` and responds `OK DEBUG REGS` followed by the register values.

#### Scenario: Read a memory location
- **WHEN** the user sends `DEBUG READ <addr>\r\n` over the CDC serial port while the target is halted
- **THEN** the probe responds `OK DEBUG READ <addr>=0x<value>` with the 32-bit value read from the target memory.

#### Scenario: Write a memory location
- **WHEN** the user sends `DEBUG WRITE <addr> <val>\r\n` over the CDC serial port while the target is halted
- **THEN** the probe writes the 32-bit value to the target memory and responds `OK DEBUG WRITE`.

### Requirement: Automated Heartbeat & Debug Verification Suite
The system SHALL provide an automated Python test harness `test/test_stm32f1_heartbeat_debug.py` that builds the heartbeat firmware, programs it via SWD flash, executes interactive Cortex-M3 debug operations over CDC, and samples the PC13 LED output on `GPIO22` (L_CH2) with the AgentLA 100 MHz logic analyzer.

#### Scenario: 5-stage automated verification
- **WHEN** the user runs `python3 test/test_stm32f1_heartbeat_debug.py /dev/ttyACM0`
- **THEN** the harness (1) builds the heartbeat firmware, (2) programs it via SWD FPEC flash, (3) halts, single-steps, reads core registers, and mutates memory, (4) resumes the CPU and verifies the PC13 LED double-pulse signal on `GPIO22`, and (5) reports all stages passing.

#### Scenario: SWD flash programming of heartbeat firmware
- **WHEN** the harness programs the built `firmware.bin` via SWD
- **THEN** the firmware is erased, written in chunks, and verified against the source binary at `0x08000000`.

#### Scenario: GPIO22 double-pulse verification
- **WHEN** the harness samples `GPIO22` (Blue Pill `PC13`) with the logic analyzer after resuming the target
- **THEN** the capture shows two 100 ms LOW pulses separated by 100 ms HIGH followed by a 700 ms HIGH rest period across a 1 Hz heartbeat cycle.