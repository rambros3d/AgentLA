## ADDED Requirements

### Requirement: Generic algorithm as primary flash implementation
The system SHALL support executing uploaded official flash algorithms in target RAM as the primary programming path, with direct register algorithms retained only as a transitional fallback.

#### Scenario: Generic path available
- **WHEN** a valid algorithm blob and matching metadata are supplied for a connected target
- **THEN** the system programs flash through the generic algorithm path and reports `OK FLASH DONE`

#### Scenario: Fallback retained during transition
- **WHEN** generic algorithm support is disabled or unavailable for a known family
- **THEN** the system falls back to the existing direct register algorithm without changing observable results