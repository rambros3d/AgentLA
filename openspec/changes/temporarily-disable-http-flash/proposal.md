## Why

The HTTP flash path is currently the least reliable surface in the programmer:

- Three parallel STM32 flash implementations exist (`firmware-rs/src/programmer/mod.rs`,
  `agentla-programmer::algo`, `agentla-targets::cascade`), and only the first is on the live path.
- The dual-bank geometry (`FLASH_CR.BKSEL`) is implemented in the live path but not in the
  cascade, and the catalog's H563 entry points at the wrong register block.
- `CortexM::connect_under_reset` arms `DEMCR.VC_CORERESET` and never clears it, so any flash
  followed by a software reset re-halts the target at the reset vector.
- The FLM/Tier-2 infrastructure exists but is not wired to `POST /api/v1/programmer/flash`, and
  the shipped `/system/algo/*.bin` files are 24-byte stubs whose every function returns success
  without writing anything.

The USB transport path is materially more mature. CMSIS-DAP passes all 12 checks in
`test/test_cmsis_dap.py`, and the ST-Link personality implements the full V2 command surface
(`0xF1`, `0xF2`/`0x30`-`0x4B`, `0xF5`, `0xF7`) that `st-flash` and `st-info` require.

Temporarily disabling the HTTP flash routes stops the vector-catch trap from being reachable
through the default API, removes the least-maintained code from the request path, and makes the
USB transport the supported way to program a target. The code is retained behind a single flag so
the capability can be restored without a rewrite.

## What Changes

- Add a build-time flag `HTTP_FLASH_ENABLED` (default `false`) in `firmware-rs/src/rest.rs`.
- Gate the HTTP flash-shaped routes on it:
  - `POST /api/v1/programmer/flash`
  - `POST /api/v1/programmer/erase`
  - `POST /api/v1/programmer/reset-catch`
  - `POST /api/v1/programmer/rescue/stm32`
  - `POST /api/v1/programmer/rescue/nrf`
- Return `501 Not Implemented` with a machine-readable body when the flag is off:

```json
{"status":"disabled","error":"http_flash_disabled","use":"usb","modes":["cmsis","stlink"]}
```

- Gate the corresponding MCP tools `programmer_flash` and `programmer_unbrick` with the same
  error shape.
- Surface the disabled state in the WebUI instead of offering a button that can only fail.
- Update `README.md`, `docs/rest-api.md`, and `skills/agentla-programmer/SKILL.md` to document
  USB ST-Link / CMSIS-DAP as the supported programming path.
- Update or skip the HTTP-flash integration tests so the suite is green with the flag off.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `rest-target-programmer`: The erase and flash routes remain published but SHALL return
  `501` with `http_flash_disabled` while `HTTP_FLASH_ENABLED` is false. The non-flash routes
  (`target`, `read`, `reset`, `rtt`, `swo`, `mode`, `stlink/version`) are unaffected.

## Impact

- `firmware-rs/src/rest.rs`: flag and guards in `ProgrammerFlash`, `ProgrammerErase`,
  `ProgrammerResetCatch`, `ProgrammerRescueStm32`, `ProgrammerRescueNrf`.
- `firmware-rs/src/mcp.rs`: guards in `programmer_flash` and `programmer_unbrick`.
- `webui/agentla-bridge.js`, `webui/app.js`: disabled-state handling.
- `README.md`, `docs/rest-api.md`, `skills/agentla-programmer/SKILL.md`.
- `test/test_bluepill_flash.py`, `test/flash_*.py`, and any test that posts to
  `/api/v1/programmer/flash` or `/erase`.
- Does not touch `crates/agentla-programmer`, `crates/agentla-targets`, the PIO engine, or the
  USB personalities. The programmer internals remain intact for when HTTP flashing is restored.

## Out of Scope

- Fixing the `VC_CORERESET` teardown (tracked in `cortex-m-debug-session-lifecycle`).
- Unifying the three flash implementations (tracked in `programmer-architecture-simplification`).
- Replacing the stub FLM blobs with real algorithms.
- Removing the routes or deleting the flash code.