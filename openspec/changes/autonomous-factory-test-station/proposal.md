## Why

In high-volume manufacturing environments, programming and functional testing of assembled PCBs is performed using automated bed-of-nails fixtures (pogo pin jigs). Currently, AgentLA's Target Station requires manual operator clicks in the WebUI to flash firmware, toggle power rails, and inspect peripherals.

This change proposes an autonomous, hands-off Factory Production and Functional Circuit Test (FCT) Station (planned for post-v1 releases). By combining BakuProbe's onboard hardware (TI INA180 current monitoring, switchable target power, level translation, and CAN-FD/RS-485 transceivers) with autonomous contact sensing, the fixture automatically detects when a PCB is clamped into pogo pins, verifies electrical safety before powering up, flashes firmware, injects serialized identity data, runs peripheral loopback tests, and displays a prominent PASS/FAIL verdict in seconds.

## What Changes

- **Automated Contact Sensing & Short Protection**: Continuous monitoring of VTARGET voltage and clamp status to initiate testing automatically upon pogo pin contact, backed by sub-millisecond overcurrent power tripping (<100mA safe limit) via INA180 to protect jigs from shorted target boards.
- **Declarative Factory Test Recipes**: Introduces a structured recipe format (`factory_recipe.json`) specifying expected chip IDCODE, binary images, flash offsets, serialization formats, and multi-instrument test sequences.
- **High-Throughput Flashing & Serial Injection**: Autonomous mass erase, high-speed flashing, and injection of unique serial numbers, MAC addresses, and calibration coefficients directly into target Flash/OTP.
- **Automated Functional Circuit Testing (FCT)**: Automated release from reset followed by UART bootlog banner verification, CAN-FD / RS-485 transceiver ping/echo tests, bus stimulus (I2C/SPI), frequency measurement via PIO, and active/sleep current draw logging.
- **Dual-Mode Operator & Headless Control**: Dedicated fullscreen "Production Station" WebUI tab with high-contrast PASS/FAIL indicators, audio chimes, and live yield counters, accompanied by a headless Python runner (`tools/production_runner.py`) for factory MES and automated testing farm integration.
- **Release Target**: **Future Architecture (Post-v1 Release)**.

## Capabilities

### New Capabilities
- `automated-contact-watchdog`: Contact sensing via VTARGET/microswitch and pre-flight sub-millisecond overcurrent short-circuit protection via INA180.
- `declarative-factory-recipes`: Standardized schema for JSON test recipes orchestrating programming, verification criteria, and peripheral pass/fail thresholds.
- `autonomous-fct-pipeline`: High-speed flashing, serialization patching, and multi-bus functional testing (UART, CAN-FD, RS-485, I2C/SPI, power profiling).
- `production-station-surface`: Fullscreen high-contrast WebUI surface with PASS/FAIL indicators, cycle timers, yield statistics, audio feedback, and headless MES export.

### Modified Capabilities
- `target-station`: Adds autonomous recipe execution mode and automatic pogo-pin trigger cycle alongside existing manual interactive debugging controls.

## Impact

- **Firmware Engine**: Adds autonomous state machine for contact detection, overcurrent trip ISR, and recipe step execution in `firmware/src/`.
- **WebUI Surface**: Adds full-screen Production Station view designed for shop-floor touch monitors and barcode scanners.
- **Tooling & Automation**: New CLI/Python tool `tools/production_runner.py` communicating with AgentLA over USB RNDIS HTTP / JSON-RPC.
- **Hardware Integration**: Utilizes BakuProbe Zone 1 (SWD/Power), Zone 2 (CAN/485 transceivers), and Zone 5 (INA180 current sense / ADC7 voltage sense).
