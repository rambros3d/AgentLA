## Context

A Cortex-M debug session is the only thing standing between the probe and a target that can
disable its own SWD pins, sleep, or lock up microseconds after reset. The repository already has
all the primitives:

| Primitive | Location | Purpose |
|---|---|---|
| `SwdSession` / `release()` / `Drop` | `firmware-rs/src/programmer/pins.rs` | Owns transport and pin tristate |
| `CortexM` | `crates/agentla-programmer/src/target/cortex_m.rs` | AP/CSW selection, core registers, `connect_under_reset` |
| `attach()` | `firmware-rs/src/programmer/pins.rs` | `try_attach` + `power_up_debug` |
| `stm32h5_configure_dbgmcu` | `crates/agentla-targets` | DBG_STOP / DBG_STANDBY / APB1 freeze |

What is missing is a **contract** for how a session ends. Today each endpoint decides for itself,
and they disagree:

| Endpoint | Attaches | HALT | RESUME | Restores DEMCR |
|---|---|---|---|---|
| `detect_target_port` | attach | no | no | no |
| `read_flash_port_internal` | attach (+CUR fallback) | no (non-intrusive) | no | no |
| `erase_flash_port_global` | attach + CUR for H5 | CUR | no | no |
| `flash_target_port_global` | attach + CUR for H5 | CUR | **yes** | **no** |
| `halt_target_port_global` | attach | yes | no | no |
| `resume_target_port_global` | attach | no | yes | no |
| reset-catch | CUR | yes | no | no |
| GDB server | try_attach | GDB-driven | **yes** | **no** |

`DEMCR` is written in exactly one place (`cortex_m.rs:362`, value `0x0100_0001`) and read nowhere.

## Goals / Non-Goals

**Goals:**
- Restore target debug state when any SWD session ends, on both success and error paths.
- Make the ARMv8-M flash path attach once, not twice.
- Keep `#![no_std]`, zero-heap, zero-emoji, and the existing `Drop`-based pin release.
- Preserve `DEMCR.TRCENA` when the caller had tracing enabled.

**Non-Goals:**
- Changing CMSIS-DAP packet handling.
- TrustZone / secure-world debug configurations.
- Restructuring the QingKe (RISC-V) session type; only the Arm path has a vector-catch equivalent.
- Changing the verify-failure behaviour of flash (currently resumes then errors).

## Decisions

### Decision 1: Separate "attach mode" from "detach policy"

These are orthogonal and today they are entangled in every call site.

```
                    attach mode                     detach policy
        ┌──────────────────────────────┐   ┌──────────────────────────────┐
        │ Normal      attach only      │   │ Leave   restore DEMCR only   │
        │ UnderReset  nRST + cycle-0   │   │ Halt    leave C_HALT set     │
        │ Auto        Normal, fallback │   │ Run     resume core          │
        │             to UnderReset    │   │                              │
        └──────────────────────────────┘   └──────────────────────────────┘
```

`SwdSession::open(mode)` performs the preamble once and records the resulting
`TargetProfile` (`Armv7M` or `Armv8M` with AP/CSW/DBGMCU) on the session.

### Decision 2: The session owns target debug state, not the call site

Add to `SwdSession`:

```rust
profile: Option<TargetProfile>,
saved_demcr: Option<u32>,
detach: DetachPolicy,
```

and extend the existing `Drop` impl:

```rust
impl Drop for SwdSession {
    fn drop(&mut self) {
        self.teardown();            // NEW: restore DEMCR, apply detach policy
        self.transport.deinit();
        tristate_pins();
    }
}
```

*Rationale*: `Drop` already guarantees pin release on early return; putting teardown next to it
guarantees target release by the same mechanism. No new type, no borrow gymnastics, no call-site
discipline. `release()` already returns `_pins` by copy and then drops, so the behaviour composes.

*Alternative considered*: a separate `TargetLease<'a, S>` guard returned from
`session.open()`. Rejected because it would need to hold `CortexM<'a, S>` while the caller still
owns `SwdSession`, forcing either self-referential structure or a second mutable borrow. `Drop` on
the session is simpler and cannot be forgotten.

### Decision 3: Teardown restores the captured `DEMCR`, not a constant

On attach (after AP/CSW selection, before any arming) read `0xE000EDFC` and store it.
On teardown, write it back. If the read failed, store `None` and write `0`.

This is the difference between "clear VC_CORERESET" and "leave the debug block as we found it".
It also naturally preserves `TRCENA` for SWO users, which a hardcoded `0` would silently break.

### Decision 4: `DetachPolicy` is set at open and may be overridden

```rust
pub enum DetachPolicy {
    Leave,  // restore DEMCR only, do not touch DHCSR
    Halt,   // restore DEMCR, then force DHCSR.C_HALT
    Run,    // restore DEMCR, then resume
}
```

Default `Leave`. `flash`/`erase`/`gdb` open with `Run`; `halt` and reset-catch open with `Halt`;
`read` and `detect` stay `Leave` so a live read never disturbs the core.

Teardown is best-effort: it performs no polling loops and cannot fail the operation. Errors are
ignored (the crate has no logging sink on all targets; the host binary can add `defmt` later).

### Decision 5: `Auto` mode is "try normal, fall back under reset"

```
open(Normal)          open(UnderReset)        open(Auto)
     |                      |                      |
  try_attach          assert nRST           try_attach
     |ok                power_up_debug            |ok -> done
     |                    select profile          |fail -> assert nRST
  done (no reset)      arm vector catch         done
                       release nRST
```

For flash/erase, use `Auto`. It attaches to a running target when possible, and falls back to
cycle-0 catch for the targets that motivated this work. This removes the current pattern of
`attach()` followed by `connect_under_reset()` on every ARMv8-M flash.

### Decision 6: Keep `resume()` semantics, add teardown separately

`CortexM::resume()` stays "clear C_HALT". `CortexM::teardown(policy)` is the new composite
operation. This keeps the GDB server's per-command `resume` intact while making detach explicit.

## Risks / Trade-offs

- **Drop cannot report errors.** If SWD is dead (target unplugged) the writes silently fail. That
  is acceptable: teardown must never mask the operation's real result.
- **Reading DEMCR on attach adds one SWD transaction.** Negligible against flash times, but it does
  mean `open()` can fail on a target that is otherwise readable. Mitigation: a failed DEMCR read
  sets `None`, it does not fail the attach.
- **`Auto` fallback doubles reset time on failure.** Bounded by the existing 500 ms halt timeout.
- **Behaviour change for `flash` on failure.** Today a verify mismatch resumes the core and then
  returns `0xDEADBEE1`; with `Run` the same happens. Making failure leave the core halted is a
  separate safety decision and is deliberately not bundled here.

## Migration Sketch

```rust
// before: 12 lines repeated at every call site
let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
let (dpidr, is_h5) = match swd_session.attach() { /* ... */ };
let mut cortex = CortexM::new(&mut swd_session.transport);
if is_h5 { cortex.select_ap(1); cortex.csw = 0x23000012; }
// ... operation ...
*pins = swd_session.release();

// after: teardown is owned by the session
let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
swd_session.set_detach_policy(DetachPolicy::Run);
swd_session.open(AttachMode::Auto)?;
let mut cortex = swd_session.cortex();
// ... operation ...
*pins = swd_session.release();   // Drop restores DEMCR before deinit
```