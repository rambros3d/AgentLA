# Slot B HTTP server fails to bind after OTA

## Symptoms

After an OTA to Slot B, the probe is reachable by ping but not by HTTP:

| Check | Result |
|---|---|
| `ping 10.10.10.10` | works, ~0.9 ms |
| USB enumeration | works, `1209:414d Generic Adi Linden` |
| Port 80 (HTTP) | **connection refused** |
| Port 3333 (GDB) | TCP accept works, never sends a banner |
| Port 5555 (OLS) | closed |
| CMSIS-DAP (12 checks) | all pass |
| `test/test_cmsis_dap.py` serial | `00000001` (pre-serial-fix firmware) |
| probe-rs SWD to target | works, reads vectors |

The probe is alive, the network stack is up, USB works, SWD works. Only the
application-level servers fail. This is a **boot-hang before the server tasks get
a working stack**, not a network or HTTP-layer bug.

## Root cause

`firmware-rs/src/main.rs` spawns the network tasks and then performs blocking
flash work on the same executor, before the watchdog exists:

```rust
spawner.spawn(web::web_server_task(stack).unwrap());     // binds :80
spawner.spawn(crate::programmer::gdb_server::gdb_server_task(stack).unwrap());
spawner.spawn(crate::ols::ols_server_task(stack).unwrap());

let flash = Flash::new_blocking(p.FLASH);
let mounted = storage::init_filesystem(flash);           // step 4

if mounted {
    rest::init_settings_from_fs();                       // step 5
    usb::load_personality_from_fs();                     // step 6
    usb::load_stlink_version();                          // step 7
    profile::scan_boot_default().await;                  // step 8  <-- BLOCKING
}

if let Err(()) = boot::ota_confirm() { ... }             // step 9
crate::programmer::pins::tristate_pins();                // step 10
let watchdog = Watchdog::new(p.WATCHDOG);
spawner.spawn(watchdog_task(watchdog).unwrap());         // step 11
```

Three properties combine:

1. **The watchdog is started last (step 11).** Anything that hangs before it
   produces exactly this symptom: no reset, no reboot, no watchdog recovery.
2. **`profile::scan_boot_default()` is `.await`ed at step 8.** It calls
   `apply_profile` which does `embassy_time::block_for(5ms)` and
   `pin_owner::quiesce_and_reset()` (which itself does `block_for(1ms)`).
   Blocking the executor stalls every other task including the just-spawned web
   server.
3. **The flash driver panics on error, it does not return `Err`.**

```rust
// firmware-rs/src/storage.rs:101
fn write(&mut self, off: usize, data: &[u8]) -> littlefs2::io::Result<usize> {
    ...
    flash_guard().as_mut().unwrap().blocking_write(abs_off, data).unwrap();
    Ok(data.len())
}
```

So a LittleFS write failure becomes a `panic=abort` in the boot path, not a
recoverable `Err`. The other two mount guards are `try_lock().unwrap()` as well
(`storage.rs:298`, `storage.rs:337`).

### Why OTA triggers it

`ota_confirm()` (step 9) writes the boot table via
`flash_boot_sector_commit`, which erases a 4 KB sector at `0x2000`/`0x3000`.
That sector is **below** `SLOT_A_OFFSET` (`0x4000`), so it is outside both
LittleFS partitions — but it is the same `FLASH` peripheral that XIP is
executing from. The embassy-rp blocking flash driver handles this by copying the
routine to RAM; if any task is mid-XIP-fetch during that window the fetch faults.

Combined with step 8's `block_for` on the same executor, the boot path has a
narrow window where a flash error panics instead of degrading.

## Why it is intermittent

- `active_slot` / `current_slot` were `SlotB` / `SlotA` before the failed boot.
  The candidate was confirmed, so `ota_confirm()` took the no-op branch.
- The pre-existing `Slot B` image already showed this: `test_cmsis_dap.py`
  reports serial `00000001`, which is *older* than the serial fix. So Slot B was
  already broken before this session's changes.
- The failure depends on whether the flash write at step 9 collides with a task
  that is actively using the stack. On a clean cold boot the timing differs from
  a warm reset after OTA, which is why it appears "after OTA" rather than always.

## Contributing factors found

### A. `usb_serial_str` uses `static mut` (introduced this session)

```rust
static mut SERIAL_BUF: [u8; 24] = [b'0'; 24];
```

It is written once and only read through raw pointers, so it is sound under the
single-threaded executor, but it is a new `unsafe` surface in the boot path.
This is **not** the cause of the HTTP failure (the running firmware predates it),
but it should be replaced with a `critical_section`-guarded `Cell` or computed
into a `StaticCell` before the next release.

### B. `profile.rs` `USER_BLOCK_COUNT` comment is wrong

```rust
pub const USER_BLOCK_COUNT: usize = 284; // Pico 2
```

The computed value is **376**, not 284. The constant is correct; the comment is
stale. Worth fixing because the next reader will trust the comment.

### C. `flash_guard()` uses `try_lock().expect(...)`

```rust
fn flash_guard() -> ... {
    let mut guard = FLASH_MUTEX.try_lock()
        .expect("FLASH mutex unavailable (init_filesystem not called or nested op)");
    ...
}
```

A nested flash operation (e.g. `ota_confirm` while a LittleFS write is in flight)
panics rather than waiting. The comment says "nested op" is unsupported, which is
accurate, but the boot path can produce one.

## Proposed fix (separate change, not yet implemented)

1. **Start the watchdog before the blocking flash work.** Spawn
   `watchdog_task` immediately after `embassy_rp::init`, not at step 11. This
   converts a hang into a reboot, which is always better than a dead probe.
2. **Move `scan_boot_default()` off the boot path.** Spawn it as a task, or run
   it after `ota_confirm()`, so `block_for` cannot stall the network stack.
3. **Make `flash_guard()` and the `Storage` impl return errors instead of
   panicking.** Replace `.unwrap()` with `.map_err(|_| LfsError::IO)`.
4. **Gate the whole post-network init block behind `mounted`.** It already is,
   but `boot::ota_confirm()` runs unconditionally and touches flash even when
   the filesystem failed to mount.

## Status

Diagnosed, not fixed. The probe is currently running a stale Slot B image whose
boot path hangs at step 8/9. Recovery requires BOOTSEL + UF2.

Once recovered, verify the fix by:
- OTA to Slot B twice in a row and confirm both boots reach HTTP
- `curl http://10.10.10.10/api/v1/firmware/info` returns 200 after every OTA
- confirming `test_cmsis_dap.py` still passes 12/12