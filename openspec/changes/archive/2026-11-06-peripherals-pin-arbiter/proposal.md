## Why

The Rust firmware (`firmware-rs`, branch `embassy`) has working storage/web (Phase 1) and CMSIS-DAP/USB, but nothing governs GPIO ownership: only static programmer tristates exist, and the servo/PWM/profile/peripheral features of the C firmware have no Rust counterpart. Port-parallel work (logic analyzer, bus bridges) all claim physical pins, so the pin-arbitration primitive must exist **before** those phases land. This change delivers that arbiter plus the peripheral trio that exercises it end-to-end (hardware PWM/servo, flash-resident profiles), with REST parity for the existing DaisyUI desktop.

## What Changes

- **New `pin_owner` module** (Rust): in-RAM GPIO claim registry with handles, capability check, atomic claim transactions, quiesce-to-safe-state on release. Simplified from C: no observer/IRQ lifecycle (dead in C), retry-loop quiesce replaced by one-shot synchronous reset.
- **New `profile` module** (Rust): 32 × 2 KiB flash profile slots at offset `0x180000` (region already reserved below the LittleFS partition), C-format parity (magic `AGLA`, CRC32 payload, `BOOT_DEFAULT` flag), virtual read-only Slot 0 Safe State, active-slot + boot-default scan at boot.
- **New `servo` module** (Rust): RP2350 hardware-PWM servo engine (50 Hz 1 µs-tick fast path + generic frequency path), 8 channels keyed by GPIO with attached min/max/continuous semantics, CPU-hour-angle mapping, **slice-ownership tracking** (fixes the C bug where adjacent pins clobber each other's divider), and a generic `/api/v1/pwm` surface (one unified engine instead of C's two).
- **Boot-flow change in `main.rs`**: extend quiesce (tristate) to the full flexible pin range, then scan/apply `BOOT_DEFAULT` profile before spawning network tasks.
- **REST endpoints added to `web.rs`** (request/response parity with the C firmware router): `/api/v1/pins/claim`, `/api/v1/pins/release`, `/api/v1/profiles`, `/api/v1/profiles/active`, `/api/v1/profiles/apply`, `/api/v1/profiles/boot`, `/api/v1/profiles/save`, `/api/v1/profiles/delete`, `/api/v1/servo`, `/api/v1/servo/attach`, `/api/v1/servo/write`, `/api/v1/servo/write_us`, `/api/v1/servo/detach`, `/api/v1/pwm`, `/api/v1/pwm/claim`, `/api/v1/pwm/set`, `/api/v1/pwm/release`.

**Explicit deviations from C** (bug fixes, not feature loss):
- **BREAKING** vs C pin map: fixed pins change from GPIO 0–9 to GPIO **4–14** (Rust 3-port programmer pinout — SWD/RVSWD/SWIO bridges); flexible pool becomes GPIO 0–3, 11, 15–24, 26–29 (last four ADC-capable).
- Conflict handling becomes explicit HTTP 409 + JSON error (C silently truncated or returned success).
- C's unused parallel CLI PWM engine (`OUTCLAIM/OUTSET`) is not ported; one engine serves both servo and generic PWM.
- Profile payload stays metadata-only (C parity: payload never parsed on apply); real payload schema deferred until the workbench state format is finalized.

## Capabilities

### New Capabilities
- `servo-pwm-control`: Hardware PWM/servo channel engine and its REST surface (`/api/v1/servo/**`, `/api/v1/pwm/**`): attach/detach semantics, angle/pulse µs mapping, generic frequency+width mode, slice-ownership arbitration, conflict reporting.

### Modified Capabilities
- `pin-ownership`: Rust embodiment — fixed pool definition changes (GPIO 4–14 pre-claimed by the 3-port programmer bridge, not GPIO 0–9); flexible pool re-bounded accordingly; observer/IRQ requirements removed (vacuous in C, no C caller ever used them); release SHALL quiesce claimed pins to safe state (pins.rs parity).
- `hardware-profile-manager`: Flash layout pinned to Rust branch layout (offset `0x180000`, directly below the 2 MB LittleFS at `0x190000`); slot-record format, Slot-0 virtual Safe State, CRC32 payload, save/delete semantics retained.
- `rest-hardware-profiles`: Rust router gains the profiles REST surface with C-parity JSON shapes; conflict/bad-slot cases return explicit 400/409 JSON errors.
- `rest-gpio-pins`: Rust router gains `POST /api/v1/pins/claim` and `POST /api/v1/pins/release` arbitration endpoints (narrow delta — full 30-pin state inspection remains out of scope for this change).

## Impact

- **Code**: `firmware-rs/src/` — new modules `pin_owner.rs`, `profile.rs`, `servo.rs`; wiring in `main.rs` (boot quiesce/profile scan), `web.rs` (router), `storage.rs` (shared `Flash` access for profile sector read/erase/program — `FLASH` peripheral is single-instance, profile ops route through a shared owner).
- **Flash map**: uses already-reserved `0x180000`–`0x190000` window; no change to the 2 MB LittleFS partition or app slots.
- **Hardware**: RP2350 hardware PWM slices (no PIO programs added); pins 0–3/11/15–29 gain dynamic claims; programmer bridge pins 4–14 become registered fixed claims instead of untracked static state.
- **Existing specs ecosystem**: updates deltas only for the four modified capabilities above; `emergency-safe-state`, `probe-pin-configuration`, `dynamic-board-manifest` unchanged (Rust boot quiesce implements `pin-ownership` blank state, its own spec untouched).
- **Out of scope**: full 30-pin state inspection REST, adapter/I3C/CAN claims (later phases), profile payload schema, web UI workbench changes.
