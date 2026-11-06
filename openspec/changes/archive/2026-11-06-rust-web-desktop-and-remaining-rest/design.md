## Context

Phase 1/2 already expose REST surface aligned with the C firmware desktop (themes + app launcher) to drive the workbench via a web desktop (CSS, icons). LittleFS mounted and can serve static files. Phase 3/4 provided placeholder capabilities.

## Goals / Non-Goals

Goals: static desktop shell wiring pre-fetched, theme modes driven by `/api/settings`, board state route, app launch data contract extension.
Non-goals: porting full workbench app desktop state (PWA).

## Decisions

- Derive all platform route extensions solely used to serve files cached under LittleFS; keep static embedded fallback for web desktop page.
- Reuse `pin_owner` registry for board state queries.
- No web view JavaScript specifics beyond current APP_JS.

## Risks

- Memory usage may creep; confirm SRAM 520 KB budget and flash 4 MB headroom.

## Migration Plan

No disk migration. Rollback is code only.