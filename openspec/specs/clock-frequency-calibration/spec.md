# clock-frequency-calibration Specification

## Purpose
Synthesize exact high-resolution clock signals using RP2350 PIO fractional clock dividers and calibrate frequency mappings across host debugging protocols (ST-Link divisor table, CMSIS-DAP Hz rate, and WCH-Link speed tiers).

## Requirements

### Requirement: Fractional PIO Clock Divider Calculation
The system SHALL compute 16-bit integer and 8-bit fractional PIO clock dividers for any requested target frequency from a given system clock, guaranteeing that the synthesized clock frequency does not exceed the requested frequency.

#### Scenario: Synthesize 4 MHz clock from 150 MHz system clock
- **WHEN** calculating divider for 4,000,000 Hz with 2 cycles per bit from 150,000,000 Hz system clock
- **THEN** system computes an integer divider of 18 and a fractional divider of 192 (18.75)

#### Scenario: Synthesize 1 MHz clock from 150 MHz system clock
- **WHEN** calculating divider for 1,000,000 Hz with 2 cycles per bit from 150,000,000 Hz system clock
- **THEN** system computes an integer divider of 75 and a fractional divider of 0

### Requirement: ST-Link Frequency Divisor Table Mapping
The system SHALL map ST-Link frequency divisor index values to safe target frequencies adhering to: index 0 maps to 4000 kHz, and index $N > 0$ maps to $\lfloor 3600 / (N + 1) \rfloor$ kHz.

#### Scenario: Map divisor index 0
- **WHEN** host sends divisor index 0
- **THEN** system maps to 4000 kHz

#### Scenario: Map divisor index 1
- **WHEN** host sends divisor index 1
- **THEN** system maps to 1800 kHz

### Requirement: WCH-Link Speed Tier Mapping
The system SHALL map WCH-Link speed tier codes to exact clock rates: tier 1 maps to 6000 kHz, tier 2 maps to 4000 kHz, and tier 3 maps to 400 kHz.

#### Scenario: Map WCH speed tier 1 (High speed)
- **WHEN** host sends WCH speed code 1
- **THEN** system configures clock to 6000 kHz

#### Scenario: Map WCH speed tier 3 (Low speed)
- **WHEN** host sends WCH speed code 3
- **THEN** system configures clock to 400 kHz
