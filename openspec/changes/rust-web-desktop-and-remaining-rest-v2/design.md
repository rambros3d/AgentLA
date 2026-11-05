## Context
Phase1 serves apps via LittleFS, fallback page works; now need more complete desktop endpoint coverage.

## Goals
Parity for desktop profile apps and the telemetry path; completion so no 404s on desktop calls.

## Decisions
- Embed the desktop template files as Rust constants with macros (one-time burn).
- Use profile settings & board manifest from AgentLA shipped static bundles in `/web/demo/board.json`.
- No new backends or storage format; LittleFS already mounted.

## Risks
- RAM cap 520KB; payload caches beyond 32KB.
