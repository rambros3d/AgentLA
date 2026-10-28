## ADDED Requirements

### Requirement: 1-Wire SDI Pulse Framing
The system SHALL generate and decode CH32V003 single-wire debug interface (SDI) pulse sequences on GPIO5.

#### Scenario: Line reset and sync pulse
- **WHEN** 1-wire SDI protocol is initialized
- **THEN** line is driven LOW for $>10\,\mu\text{s}$ and released HIGH for $>2\,\mu\text{s}$

#### Scenario: Pulse width bit cell encoding
- **WHEN** data bits are transmitted on SDI
- **THEN** high-to-low pulse width ratios encode logic 0 and logic 1 bits accurately
