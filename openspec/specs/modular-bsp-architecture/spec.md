# modular-bsp-architecture Specification

## ADDED Requirements

### Requirement: Shared Board Support Package (BSP) Trait
The `agentla-bsp` library SHALL define a `BoardProfile` trait that establishes standard, type-safe hardware contracts for pin zones, programmer interfaces, transceivers, flexible instrument pools, and analog front-ends.

#### Scenario: Board crate implements BoardProfile
- **WHEN** a board crate implements `BoardProfile`
- **THEN** it provides compile-time constants and functions for `programmer_pins()`, `adapter_pins()`, `flexible_pool()`, `analog_frontend()`, and `features()`.

### Requirement: Reusable Front-End IC Drivers
The `agentla-bsp` library SHALL include reusable, hardware-agnostic drivers for external ICs including the TI PGA113 programmable gain amplifier and TI INA180 current sense amplifier.

#### Scenario: Setting PGA113 analog gain
- **WHEN** the firmware requests gain magnification (e.g. 10x, 50x, 100x, 200x)
- **THEN** the `Pga113` driver transmits a 16-bit SPI command `0x2Axx` to the chip select pin and updates active analog scaling factors.
