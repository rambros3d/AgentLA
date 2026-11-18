# Slot B HTTP failure: root cause and fix

> **HISTORICAL / STALE (2026-11-18):** Slot B and the A/B boot table no longer exist. The watchdog-startup finding below still applies, but the slot offsets and candidate/rollback reasoning do not describe the current single-slot firmware.

## Symptoms

After an OTA to Slot B the probe was reachable by ping but not by HTTP:

| Check | Before | After |
|---|---|---|
| `ping 10.10.10.10` | works | works |
| USB enumeration | works | works |
| Port 80 (HTTP) | **connection refused** | 200 |
| Port 3333 (GDB) | TCP accept, no banner | open |
| Port 5555 (OLS) | closed | open |
| CMSIS-DAP 12 checks | pass | pass |
| Target detect | not reachable | STM32H503 detected |

## Three independent causes

### 1. Watchdog started last

`main()` spawned the watchdog after all blocking work:

```
spawn(web, gdb, ols)      <- :80 binds
storage::init_filesystem
init_settings_from_fs
load_personality_from_fs
load_stlink_version
scan_boot_default().await  <- blocking
ota_confirm()              <- flash erase
tristate_pins
spawn(watchdog_task)       <- TOO LATE
```

Any hang before the last line left the probe alive with no HTTP and no way to
recover except power-cycling. A hang is indistinguishable from a slow boot
without the watchdog running.

**Fix:** spawn the watchdog immediately after `embassy_rp::init`, and widen the
window to 8 s with a 2 s feed so a progressing boot is not reset.

### 2. `scan_boot_default()` blocked the executor

It calls `apply_profile` -> `quiesce_and_reset` -> `embassy_time::block_for`,
which stalls the single-core executor including the just-spawned web server.

**Fix:** spawn it as its own task, off the boot path.

### 3. Flash `Storage` impl panicked instead of returning `Err`

```rust
flash_guard().as_mut().unwrap().blocking_write(abs_off, data).unwrap();
```

A flash error became `panic=abort` in the boot path. The mount guards used
`try_lock().unwrap()` too.

**Fix:** map flash errors into `littlefs2::io::Error::IO`. Also gated
`ota_confirm()` on `mounted`, since it erases a boot-table sector on the same
FLASH peripheral XIP executes from.

## A fourth bug found while verifying: slot detection

`current_executing_slot()` tested only the upper bound:

```rust
if vtor >= 0x1010_4000 || (pc >= 0x1010_4000 && pc < 0x1020_4000) {
    SlotId::SlotB
} else {
    SlotId::SlotA
}
```

Slot A's XIP range is `0x10004000..0x10104000`, so a Slot A PC is below both
bounds and the function returned `SlotB`. `.other()` then returned Slot B and
**OTA overwrote the running slot**.

Fixed by matching each slot range explicitly, treating VTOR as authoritative,
and using the PC only when VTOR is unknown. Five host tests added under
`crates/agentla-boot-proto` (`cargo test -p agentla-boot-proto`: 9/9).

## A fifth bug found while verifying: two serials

`DAP_Info(0x03)` in `crates/agentla-programmer/src/dap.rs` hardcoded
`"00000001"`, independent of the USB string descriptor. Fixing one left the
other stale. Added `set_serial` and wired both from `create_composite_usb`.

## Deployment trap

The OTA handler picks the target slot from `current_executing_slot().other()`.
If the image was linked for the *other* slot (`--features slot-a` vs `slot-b`),
the bootloader rejects it and falls back to the slot holding the older build.
Three "successful" OTAs changed nothing because of this.

Check with:

```
arm-none-eabi-readelf -h target/thumbv8m.main-none-eabihf/release/firmware-rs | grep Entry
```

Slot A links at `0x10004xxx`, Slot B at `0x10104xxx`.

## Verification

All four fixes plus the two adjacent bugs are live on the probe:

```
active_slot=SlotB  current_slot=SlotB   (consistent)
USB serial  : C000DDC77247F500414D0001  (24 chars)
DAP serial  : C000DDC77247F500414D0001  (matches)
CMSIS-DAP   : 12/12
flash/erase : 501 + http_flash_disabled
read        : F87F0020 41020008
i2c scan    : 47 ms
```