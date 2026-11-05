## Context

Branch `embassy`, crate `firmware-rs`. Phase 1 (committed `967fa0d`) proved the shared-pattern stack on hardware: `Mutex<CriticalSectionRawMutex, _>` registries (`FS_MUTEX`), `with_fs` style closures, heapless JSON responses in picoserve handlers, blocking `embassy_rp::flash` writes with page-burst discipline, and 10/10 hardware verification. All of Phase 2 reuses these idioms.

Current pin reality (verified source):
- Programmer bridge statically owns GPIO 4–14 via `programmer/pins.rs` (`tristate_pins()`, `ProgrammerPins` move-token with Drop→tristate). Pin 11 is deliberately excluded from the tristate list.
- LED on GPIO 25 (`embassy_rp::gpio::Output`, boot-early).
- C references: `pin_owner.c` (381 L), `profile_manager.c` (284 L), `servo_pwm.c` (260 L), plus REST handlers in `rest_api.c` (~640–770, ~1000–1120 regions) and their spec files under `openspec/specs/` (`pin-ownership`, `hardware-profile-manager`, `rest-hardware-profiles`, `rest-gpio-pins`).
- C quirks found in exploration (subagent study, 2026-11-05): observer path unusable; profile payload always `"{}"` and never parsed on apply; adjacent servo pins silently clobber each other's slice divider; two parallel PWM APIs; profile save cif corrupts neighbor slot on mid-erase power loss; single 1375 B scratch buffer silently truncates lists.

## Goals / Non-Goals

**Goals:**
- Rust `pin_owner` registry with handles, capability check, conflict-safe atomic claims, quiesce-on-release.
- Flash profile storage at `0x180000` in C slot-record format (binary-compatible), boot-default apply, metadata-only payload (C parity).
- Unified hardware-PWM servo + generic PWM engine at register level with slice-ownership tracking.
- REST surface with JSON parity to C shapes (workbench desktop already speaks them), explicit error statuses.
- Boot flow: quiesce flex range → scan `BOOT_DEFAULT` → record active slot.
- Build gated by `cargo build --release`; hardware verification via Phase 1's hands-free loop.

**Non-Goals:**
- Full `/api/v1/pins` 30-pin inspection/mode/write surface (later; needs its own design pass on borrow patterns).
- Adapters (I2C/CAN/I3C/LED) claims — they *consume* the arbiter after this change.
- Logic analyzer capture bank claims.
- Profile payload schema (stays `"{}"`-metadata; deferred until workbench state format is agreed).
- Web desktop UI changes.
- Journaling for profile sector (`0x180000`) — accepted risk, C parity.

## Decisions

1. **Pin registry type**: one `static PIN_REGISTRY: Mutex<CriticalSectionRawMutex, PinRegistry>` with `SlotId(u32)` handles.
   - *Why*: matches Phase 1 idiom; claims are short critical sections; the only mutator discipline needed.
   - *Alternative*: embassy `blocking_mutex` spinlock — no benefit; CriticalSectionRawMutex already in crate.
   - Claims mutate dpin state through raw `pac::PADS_BANK0`/`IO_BANK0` writes (same register-level style as `programmer/pins.rs`), because embassy typed `Output`/`Flex` bars re-claiming a pin you didn't start with.

2. **Pin maps** (explicit constants, not C's):
   - `FIXED_PINS: [4..=14]` — always "claimed" by `PROGRAMMER` at registry init; registry denies drives with `CONFLICT` (409).
   - `FLEX_PINS: [0,1,2,3,11,15..=24,26..=29]` — claims allowed; capability bitmask per pin (SPI/UART/I2C/PWM/SIO/PIO0-2; 26–29 also ADC).
   - Pin 25 (LED), 30/31 excluded (USB pad on 30/31 for Pico 2 board).
   - *Why*: matches the deployed 3-port hardware reality; the C ranges (0–9 fixed) are wrong for this board.
   - *Alternative*: dynamic probe of board — no board-detect mechanism exists; overkill.

3. **Observer concept dropped**: no `observer_only` claims, no IRQ attach/detach. C has zero production callers.
   - *Why*: dead code; the "coexist with active driver" pattern can be reintroduced as a narrow `listen` claim type when a real consumer appears (e.g., analyzer trigger inputs), behind a spec delta at that time.

4. **Quiesce semantics**: release takes `SlotId` (never pin indices); after removing the claim, every freed pin is set funcsel SIO, direction IN, pulls none — the exact policy `pins.rs` uses. No per-role reset variety (C let callers improvise). No async waiting: quiesce is synchronous register writes with a fixed 1 ms settle timer after the last write (rail discharge parity where the caller cares).
   - *Why*: C's `QUIESCING` state machine was vestigial (synchronous, unobservable); simplification buys bug-free release while still protecting module-boot flow.

5. **Profile system**:
   - Shared `Flash` singleton route: `storage.rs` keeps `init_filesystem` rejecting the shared `Flash` handle, but builds `FlashStorage`-equivalent *and* a `ProfileHal` pointing to `PROFILE_FLASH_OFFSET 0x180000`, 64 KiB window `(32 × 2048)`; reads always XIP zero-copy (`FLASH_BASE + off`, tested in Phase 1); writes go `blocking_write`, deletion goes `blocking_erase` on the slot's parent 4 KiB sector, all under a short critical section via the existing mutex pattern.
   - Not going into `littlefs2`'s domain: profile window lies outside partition bounds; safety via region-disjointness checks in the driver guard.
   - Format compatibility: magic `AGLA` u32, `version:1` u16, flags u16 bits `IN_USE|BOOT_DEFAULT|READ_ONLY`, name `[u8;32]`, CRC32 (init 0xFFFFFFFF, reflected poly 0xEDB88320, final xorout), payload_len u32, payload `[u8;2000]`.
   - CC-I32 software CRC (no hardware CRC on RP2350); implemented as `crc32(payload)`, called from profile-only code, ~140 cycles per 2 KiB block — negligible.
   - `apply_slot(safe slot)` → set active slot fields, drive NO pins (C parity). Profile application stays a metadata marker; profile content application belongs to a future change once payload schema exists.
   - Boot default scan: `main.rs` after `init_filesystem`, before `tristate_pins()`/watchdog: quiesce flex range → scan slot 1..32 for `(IN_USE && BOOT_DEFAULT)`; record. Ordering with watchdog matters: scan reads are XIP (fast); the rare silent re-boot case must not race with watchdog-fed erase paths — keep erase-in-boot only for `save_slot` REST path.

6. **PWM/servo at pac register level** (`embassy_rp::pac::PWM`): directly touch `AWSM`/`CTR` semantics only via existing `embassy_rp::pwm` where possible, but pin→slice mapping (`pin/2`, `pin%2`) requires dynamic slice selection that embassy-rp 0.10 `Slice::new`'s central type-level pin model doesn't express.
   - Decision: per-pin `pac::PWM` slices claimed through `servo` module with **slice-ownership table** (one GPIO per slice, like C's `pin/2 slice`, prevents silent clobber), driver claims BR/counter configured from the servo module.
   - *Alternative*: `embassy-rp::pwm` typed slices with a fixed pin set chosen at boot (compile-time) — works if we ban dynamic servo pins, but conflict with REST-inspectable pin claims.
   - Track `pwmslice_state: [SliceState; 16]` (one per slice), each recording `owner: Option<PinGlb>` and `funcoal: Funcoal`; one function per claiming module.
   - 50 Hz fast path: `clkdiv = 150 (int only)`, `wrap = 19999`, `level = pulse_us` (µs tick at 150 MHz sys_clk: `sys_clk/div = 1 MHz`); clamps pulse to `[min,max]`.
   - Generic path: `div = sys/(freq·65536)` clamp 1..255; `wrap = sys/(freq·div) − 1` clamp 1..65534; `level = (wrap+1)·pulse_us/period_us` clamp `[0, wrap+1]` — crit-equivalent of C, with the addition that a failed claim leaves the slice unmodified (no writes on conflict, C could clobber).
   - Continuous (throttle) mode: −100..+100 with ±5 deadband → pulse mapped `min..1480` / neutral / `1520..max` (C semantics verbatim).

7. **REST layer**: each handler builds its own `heapless::String<N>` (sized per endpoint: profiles list 2048, servo list 512, others ≤256) — no shared scratch. Errors are explicit `(StatusCode::NOT_ACCEPTABLE / CONFLICT, ("Content-Type","application/json"), …)` responses. No truncation possible silently; a full buffer returns an explicit 500 `overflow` error instead (catchable, testable).
   - *Why*: Phase 1 established per-handler buffer style; eliminates C's biggest correctness bug (silent list truncation).
   - Router order: parametric routes registered before literal siblings (Phase 1 lesson with `/api/v1/apps/{name}`).

8. **Deviation on response shapes**: keep C JSON key names verbatim (`active_slot`, `active_name`, `slots[]`, `angle`, `pulse_us`, …) so the DaisyUI desktop renders without changes; only ENrich error bodies with a `{"error":"…"}` field (C used ad-hoc strings).

9. **Unified PWM surface**: `/api/v1/pwm/set` uses either `servo` channel (if the pin has servo attach) or pure PWM duty mode sharing one engine; C's CLI `OUTCLAIM`/`duty%` generator merged into the same slice table. `/api/v1/pwm/claim` errors with 409 when the pin is busy.

10. **Profile format**: keep C slot record **exactly** (name, CRC, flags) for forward compat with tooling reading the flash region offline (`picotool load`-adjacent custom tooling can inspect slot table). Slot are 2048 B; erase is at 4 KiB sector granularity (2 slots/sector), so `save_slot(slot)` reads the sector, mutates one half, erase+reprogram — same as C, including the documented neighbor-corruption-on-power-loss risk (accepted; documented in TESTING.md).
    - *Why format compatibility*: lets a desktop app read profiles from a `.uf2` backup; keeps a future path open to persist actual pin layouts.

## Risks / Trade-offs

- [Pac-register PWM: reg-touching risks errata like the C divider-clobber bug, but now with explicit `SliceState` ownership] → Mitigation: one global `slice_state` array checks ownership in the claim-free path; `{ claim: Error::CONFLICT, ... }` on `servo_claim(pin)` conflicts; `debug_assert!` against intermediate states; unit tests for the slices math against known divider formulae.
- [Profile save erase takes the full 4 KiB sector while XIP is disabled stalls everything] → Mitigation: profile writes go through short critical-section `blocking_write`/`blocking_erase` bursts using the `Flash` singleton guard; watchdog feed ceremonially extended to 8 s window around any profile write; erase performed only in the REST save path, not at boot (banned at application start).
- [Flash peripheral single-owner: shared access by storage.rs and profile mocks must not deadlock] → Mitigation: profile code takes ownership through `Flash` handle given at init, re-uses the SAME blocking-write discipline (no nested mutex); `Mutex` accessed only by profile module; littlefs never touches offsets ≥ `0x180000`.
- [Changing "fixed" from 0–9 to 4–14 breaks spec `pin-ownership`] → Mitigation: delta spec included in this change (`specs/pin-ownership/spec.delta.md`), stating the *board-specific* fixed range rationale and the mapping of old flex roles onto the new flexible pool.
- [Hardened claims could trip up existing "we always use the pins" code separators, e.g. programmer bridge touching any re-flex pins when resuscitating after a debug session] → Mitigation: programmer keeps `ProgrammerPins` token; registry **is aware of** (grey-lists) 4–14 through programmer claims but does not own/tri-state them; the registry's fixed claims are always worked through the programmer's bridge constant at `pins.rs` — one source of truth for the bridge pin list, imported once.
- [Boot ordering hazard: profile scan before `init_filesystem` might race watchdog, and LittleFS formatting could run simultaneously with profile reads] → Mitigation: profile scan is read-only XIP; it never contends with littlefs; `main` boot order is: pins quiesce → watchdog task spawn → profile scan → LittleFS mount → web.

## Migration Plan

Single-firmware change on `embassy` branch; no persistent data migration (LittleFS partition untouched; profile window starts empty). Rollback = revert commits; flash region `0x180000..0x190000` was already reserved.

## Open Questions

- Profile apply drives no pins (C parity) — acceptable for the workbench today; a future change needs the actual payload schema (pin/claim layout blobs) and would revise `hardware-profile-manager` again.
- Safe-state CLI entrypoint: mirror C's `PINOWNER`/`SAFEDISCOVER` CLI verbs? Out of scope for REST parity, deferred to a CLI phase.
- Should `/api/v1/pwm` reflect servo channels or hide them? Decided: reflect (C did the same via direct slice introspection).
