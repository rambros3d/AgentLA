# Proposal: Dynamic Pin Ownership and Flexible Adapter/Capture Routing

## Why

AgentLA’s current flexible roles are effectively welded to compile-time pin macros. Adapter mode and logic-analysis/scope support are therefore hard to retarget, test matrix execution is brittle, and automated testing cannot safely adapt to boards whose exposed interfaces live on different physical pin sets. Programmer pins must remain fixed, but the non-programmer roles should be dynamically claimable, validated, and released.

## What Changes

- Replace the compile-time “default pin” model for flexible roles with a runtime ownership registry.
- Reserve GPIO0 through GPIO9 as fixed Programmer resources in every mode.
- Allow Adapter, Logic Analyzer, Scope, and ADC roles to claim only GPIO10 through GPIO29.
- Require explicit claims before flexible-role use, with no implicit default claims and **BREAKING** changes to legacy command assumptions that hard-code adapter and capture pins.
- Add per-pin capability validation using the RP2350 rev function table and package constraints.
- Add atomic role claims, ownership handles, conflict reporting, explicit release, and automatic quiesce/release behavior.
- Add span-based capture claims: a caller declares a subset of capture pins and reserves the full `[min,max]` physical span as exclusive capture ownership.
- Support one active capture bank at a time, but permit the bank to be located anywhere within the flexible pin pool and sized to its span.
- Treat ADC/scoped functionality as a dependent capability, validating only the analog-capable subset of flexible pins.
- Embed ownership information in captures so hosts receive the actual pin-to-bit mapping and sample width, rather than assuming fixed channels.
- Update validation and error reporting so failed claims retain prior state.

## Capabilities

### New Capabilities
- `pin-ownership`: Ownership registry, resource claims, atomic validation, release semantics, conflict reporting, and command/API handle model for fixed and flexible pins.
- `capture-bank-layout`: Span-based, dynamically located capture banks with explicit channel masks, sample widths, trigger pins, and bank lifecycle management.

### Modified Capabilities
- None.

## Impact

- Affected code: `firmware/src/board_settings.h`, `firmware/src/adapter/*`, `firmware/src/analyzer/capture.*`, `firmware/src/structs.h`, and command dispatch in `firmware/src/main.c`.
- Affected APIs: adapter I2C/UART/SPI commands, capture configuration commands, scope commands, and host SDK interfaces that assume fixed adapter or analyzer pins.
- Affected system: pin-mapping assumptions, host test scripts, and documentation in `docs/pin-allocation.md`.
- Dependencies: RP2350 pin-function capability tables and package-specific analog pin availability.
- Risk: larger refactor and test rework; conflicts must be reported before any pin function is changed.
