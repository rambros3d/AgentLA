## MODIFIED Requirements

### Requirement: Universal target identification
The system SHALL identify a connected target's vendor, family, protection level, and factory geometry before any destructive operation by scanning ARM CPUID, DBGMCU_IDCODE, or RISC-V MARCHID/IDCODE against the 200 silicon families and compressed 10,000-part catalog.

#### Scenario: Identify catalog target
- **WHEN** a target is connected via SWD, RVSWD, or SWIO and `TARGET IDENTIFY` is issued
- **THEN** the system matches against the 10,000-part catalog and reports exact commercial part number, vendor, family, flash base/size, RAM base/size, assigned direct engine, and protection level.

#### Scenario: Unknown target fallback
- **WHEN** identification fails to match a known vendor/family descriptor in the catalog
- **THEN** the system falls back to generic CoreSight / RISC-V DMI memory interrogation and permits Tier 2 (LittleFS) or Tier 3 (Host FLM) dynamic flashing while forbidding unverified direct register operations.
