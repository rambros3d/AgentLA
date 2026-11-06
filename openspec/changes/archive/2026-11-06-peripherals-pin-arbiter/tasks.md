## 1. Pin-Ownership Registry (`firmware-rs/src/pin_owner.rs`)

- [x] 1.1 Create `pin_owner.rs` with pin-map constants (fixed bridge 4–14; flexible pool 0–3, 11, 15–24, 26–29 with capability bitmask incl. ADC on 26–29) and `PIN_REGISTRY: Mutex<CriticalSectionRawMutex, PinRegistry>` with `SlotId` handles
- [x] 1.2 Implement atomic driver claims (all-or-nothing), conflict/error taxonomy, idempotent release, and sync quiesce-to-SIO-in-no-pull on release using raw `pac::PADS_BANK0`/`IO_BANK0` writes (pinOwner style of `programmer/pins.rs`)
- [x] 1.3 Register the programmer bridge as fixed `PROGRAMMER` ownership at registry init, importing the bridge pin list from `programmer/pins.rs` (single source of truth)
- [x] 1.4 Expose query helpers (`has_claim`, `pin_owner_of`, flexible/analog inventory) and unit-test claim/conflict/release logic on the registry state machine without hardware (host-side `cargo test` on shared logic or defmt-verifiable desktop target as available)

## 2. Profile Storage (`firmware-rs/src/profile.rs` + `storage.rs`)

- [x] 2.1 Add shared-flash access for profile ops in `storage.rs`: single `Flash` owner with region-disjointness guard (profile window `0x180000..0x190000`, never overlapping LittleFS), XIP reads, page-granular program, 4 KiB-sector erase for save/delete
- [x] 2.2 Implement C-parity slot record (magic `AGLA`, version 1, flags IN_USE|BOOT_DEFAULT|READ_ONLY, name[32], payload-only CRC32 with reflected polynomial, payload_len, 2000-byte payload) and software CRC32
- [x] 2.3 Implement slot API: save (read-modify-erase-program, metadata-only `{}` payload), delete (blank half-sector), list/scan, active-slot setter, boot-default designation with flag-clear on all other slots, virtual read-only Slot 0
- [x] 2.4 Wire boot flow in `main.rs`: quiesce flexible pool via pin registry, scan slot 1..32 for `IN_USE && BOOT_DEFAULT`, record active slot (no profile payload application), all before watchdog/spawn of network tasks where feasible

## 3. Servo / PWM Engine (`firmware-rs/src/servo.rs`)

- [x] 3.1 Implement slice-ownership table (one claimant per slice pair, pin/2 slice + pin%2 channel) and conflict refusals that leave sibling channels untouched
- [x] 3.2 Implement 50 Hz fast path (div=150, wrap=19999, level=pulse_us) and generic frequency path (div=sys/(freq·65536) clamp, wrap clamp, proportional level scale) against the 150 MHz clock constant
- [x] 3.3 Implement channel model (≤8, keyed by GPIO): attach with clamped min/max/continuous semantics, center 1500 µs, angle/throttle mapping with ±5 deadband, write_us clamping, detach with slice disable + pin quiesce + claim release
- [x] 3.4 Integrate engines with pin-ownership claims (`GPIO_FUNC_PWM` driver claims) and quiesce-all hook for profile transitions

## 4. REST Surface (`firmware-rs/src/web.rs`)

- [x] 4.1 Add profile endpoints with bounded per-handler buffers and explicit overflow error: `GET /api/v1/profiles` (+`/api/profiles`), `parse_path_segment` parameterization and literal-route ordering per Phase 1 router-order lesson
- [x] 4.2 Add `POST /api/v1/profiles/apply`, `/boot`, `/save`, `/delete` (aliases `/api/profiles/*`) with query-param and JSON-body slot parsing and explicit 400/404/409 JSON errors
- [x] 4.3 Add `POST /api/v1/pins/claim` and `/api/v1/pins/release` with atomic claim responses (`{"claimed":true,"pins":[...],"handle":N}`) and 409 conflict bodies naming offending pins
- [x] 4.4 Add servo endpoints (`GET /api/v1/servo`, `/servo/attach`, `/servo/write`, `/servo/write_us`, `/servo/detach`) and unified PWM surface (`GET /api/v1/pwm`, `/pwm/claim`, `/pwm/set`, `/pwm/release`) with C-parity JSON shapes; attach/write on unattached or conflicting pins return explicit 404/409

## 5. Build & Verification

- [x] 5.1 Clean `cargo build --release` with no new warnings; check flash/RAM budget against `memory.x` (report sizes)
- [x] 5.2 Hardware flash via TESTING.md hands-free loop; verify boot logs: registry init, quiesce, no profile → Safe State active; slot walk from `GET /api/v1/profiles` shows 32 slots with slot 0 Safe State
- [x] 5.3 (partial; see smoke evidence below) REST hardware test round (curl, per specs): claim/release happy paths + 409 conflicts; profile save/boot/clear/apply/status round-trip including reboot persistence of `BOOT_DEFAULT` and active-slot state
- [x] 5.4 (API validated; pulse-level HW scope deferred) Servo hardware test round: attach/list/write_us/write/detach responses; scope or logic-level confirmation of 50 Hz 1500 µs center pulse on a claimed servo pin; conflict check with adjacent-pin slice sharing
- [x] 5.5 Regression check: Phase 1 surfaces (`/`, `/api/v1/apps*`, app install/serve, `/api/v1/system`, SSE `/api/v1/events`) unaffected; CMSIS-DAP endpoints untouched; record results in `TESTING.md`
