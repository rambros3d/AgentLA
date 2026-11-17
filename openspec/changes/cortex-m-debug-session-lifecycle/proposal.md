## Why

Every Cortex-M entry point under `firmware-rs/src/programmer/**` open-codes the same attach
preamble (open SWD session, attach, read DPIDR, select AP 0/1, set CSW, configure DBGMCU), and
**none of them restore the target debug state when they finish**.

`CortexM::connect_under_reset` arms `DEMCR.VC_CORERESET` (`0xE000EDFC = 0x0100_0001`) and no
code path in the workspace ever writes that bit back to zero. The flash workflow then calls
`cortex.resume()`, which writes `DHCSR = DBGKEY | C_DEBUGEN` and lets the core run with vector
catch still armed.

On the STM32H503 bench target this means:

```
flash -> resume -> BareBoot runs -> application_jump() -> NVIC_SystemReset()
                                                          |
                                                          v
                                        CoreSight catches reset -> halt at 0x08000240
```

BareBoot can therefore never launch the N20-Servo application after a probe-driven flash.
Recovering the bench required an out-of-band host write of `0x00000000` to `0xE000EDFC`, which is
not represented anywhere in this repository. The repo cannot reproduce the end state that the
handoff records as hardware-verified, and the next flash through
`POST /api/v1/programmer/flash` will re-arm the trap.

The same preamble is duplicated across eight call sites, and the ARMv8-M path attaches to a
running target first and then resets it anyway (`attach()` followed by `connect_under_reset()`),
which is the exact failure mode the under-reset path exists to avoid.

## What Changes

- Add a target debug **detach policy** to `crates/agentla-programmer` (`DetachPolicy` and
  `CortexM::teardown`) so "let user code run" and "detach cleanly" stop being the same operation.
- Make `SwdSession` (in `firmware-rs/src/programmer/pins.rs`) own the target debug state: capture
  `DEMCR` on attach, apply the policy on drop, and release pins afterwards. Every early return and
  error path inherits correct teardown for free.
- Introduce an explicit attach mode (`Normal`, `UnderReset`, `Auto`) to replace the duplicated
  attach / dual-attach cascade, and remove the double attach on the ARMv8-M flash path.
- Migrate the Cortex-M call sites: `detect_target_port`, `read_flash_port_internal`,
  `erase_flash_port_global`, `flash_target_port_global`, `halt_target_port_global`,
  `resume_target_port_global`, the reset-catch endpoint, and the GDB server.
- Add host unit tests that assert `VC_CORERESET` is cleared on teardown, that `DEMCR.TRCENA` is
  preserved when it was previously set, and that teardown runs on error paths.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `cortex-m-vector-catch`: Connect-under-reset must snapshot `DEMCR` and restore it when the debug
  session ends; add a teardown requirement covering flash, GDB disconnect, and halt endpoints.
- `domain-crate-programmer`: Add the `DetachPolicy` / `CortexM::teardown` API and require the
  firmware session layer to invoke it on drop.

## Impact

- `crates/agentla-programmer/src/target/cortex_m.rs`: `DetachPolicy`, `CortexM::teardown`,
  `DEMCR` snapshot/restore.
- `crates/agentla-programmer/tests/programmer_tests.rs`: teardown regression tests.
- `firmware-rs/src/programmer/pins.rs`: `SwdSession` debug-state fields and `Drop` hook.
- `firmware-rs/src/programmer/mod.rs`: `detect_target_port`, `read_flash_port_internal`,
  `erase_flash_port_global`, `flash_target_port_global`, `halt_target_port_global`,
  `resume_target_port_global`.
- `firmware-rs/src/programmer/target/mod.rs`, `firmware-rs/src/programmer/gdb_server.rs`.
- End-to-end: flash `n20-servo.bin` to STM32H503, confirm the application boots from BareBoot
  without any manual `DEMCR` write, and confirm the I2C target at `0x40` answers.

## Out of Scope (tracked separately)

- Dual-bank STM32H5 sector erase (`FLASH_CR.BKSEL`) has no requirement in `flash-algorithms`.
  The current text says "8 KB sector erase", which is only true for bank 1.
- The SWD write-packet turnaround (`turnaround + 3 + turnaround`) is not described by
  `swd-engine` or `swd-turnaround-timing`.
- The 7.2 s blocking sensor scan in `../N20-Servo/firmware/src/main.rs` that delays the I2C target
  server on every boot.