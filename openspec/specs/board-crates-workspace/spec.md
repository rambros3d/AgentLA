# board-crates-workspace Specification

## ADDED Requirements

### Requirement: Independent Board Crates
The repository SHALL organize supported hardware targets into independent board crates under `boards/` (`boards/pico2`, `boards/bakuprobe`, `boards/templates`).

#### Scenario: Compiling with specific board target
- **WHEN** building `firmware-rs` for a specific board target
- **THEN** only the selected board crate is compiled, preventing pin definition collisions and ensuring zero unused driver code is included.

### Requirement: Extensible Board Template
The system SHALL provide a `boards/templates` boilerplate crate allowing developers to define pin mappings, zone boundaries, and analog front-ends for custom carrier boards.

#### Scenario: Creating a custom hardware target
- **WHEN** a developer copies `boards/templates` to a new board directory
- **THEN** the crate can be built and linked into `firmware-rs` without modifying core firmware or instrument files.
