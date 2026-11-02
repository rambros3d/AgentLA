## ADDED Requirements

### Requirement: DMX-512 Frame Synchronization and Channel Extraction
The firmware SHALL detect DMX-512 frame boundaries using the Break ($\ge 88\,\mu\text{s}$) and Mark-After-Break (MAB $\ge 8\,\mu\text{s}$), verify Start Code `0x00`, and extract channel values for slots 1 through 512.

#### Scenario: Decoding DMX-512 dimmer channels
- **WHEN** a 250 kbps DMX-512 packet with Break $92\,\mu\text{s}$, MAB $12\,\mu\text{s}$, Start Code 0x00, and 12 channel values is captured
- **THEN** the decoder extracts values 0–255 for each channel slot and registers a `DMX_FRAME` event
