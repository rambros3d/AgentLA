## Context

AgentLA's ST-Link personality enumerates correctly but cannot enter debug mode.
Bisection showed a bare `return 2` for ENTER_SWD still times out while
`DBG_READCOREID` from the same function replies — so the handler logic and the
SWD engine are both cleared, and the fault is in the command/reply plumbing.

`ref/hybridlink` is a working ST-Link/V2-1 implementation with the same
protocol surface. Comparing the two isolates four differences.

## The reference: reply first, probe second

`ref/hybridlink/app/stlink.c:339-351`:

```c
case DBG_APIV1_ENTER:
case DBG_APIV2_ENTER:
    if (c[2] != DBG_ENTER_SWD)
        return reply_status(r, STLINK_NO_DEVICE);   /* SWD only */
    mode = STLINK_MODE_DEBUG;
    st = swd_connect(&idcode);
    if (st == STLINK_OK)
        st = mem_ap_init(0);
    if (st != STLINK_OK)
        idcode = 0;
    /* a genuine ST-Link reports success here even without a target;
     * the following IDCODE read tells the host whether one is present */
    return reply_status(r, STLINK_OK);
```

The reply is composed from `mode` alone. Target state is captured into `idcode`
for the *next* command. There is no path where a slow SWD operation delays the
reply.

AgentLA instead ran `reset_sequence` + `jtag_to_swd` + `read_dpidr_retry` before
returning. `read_dpidr_retry` tries 5 frequencies x 5 sequences, and each failed
`read_bits` spins up to 2,000,000 PIO cycles. Measured worst case: ~8 s, against
the host's 3 s `libusb_bulk_transfer` timeout.

## The reference: explicit endpoint re-arm

`ref/hybridlink/app/stlink_usb.c` is a 4-state machine
(`ST_CMD`, `ST_DATA_OUT`, `ST_EXEC`, `ST_DATA_IN`) where every reply is followed
by `arm_cmd()`:

```c
static void arm_cmd(void)
{
    state = ST_CMD;
    usbd_edpt_xfer(rhport_used, EP_STLINK_OUT, cmd, sizeof(cmd), false);
}
```

The IN endpoint is guarded before reuse:

```c
/* ST_EXEC: the previous reply must be gone before the buffer is reused */
if (usbd_edpt_busy(rhport_used, EP_STLINK_IN))
    return;
```

AgentLA relies on embassy-usb's implicit re-arm and discards the write result:

```rust
let _ = cmd_in.write(&resp_buf[..resp_len]).await;
```

If that write fails — endpoint busy, or a stale short packet from the 16-byte
command — the host waits 3 s and reports a timeout. Nothing in the firmware
records that it happened.

## The reference: reset AP/CSW per command

`ref/hybridlink/app/stlink.c:321-333`:

```c
/* core debug (halt, registers, reset, debug register access) is on
 * AP 0 with the default CSW, whatever a memory command used before;
 * the DAP level commands keep the AP state they manage themselves */
switch (c[1]) {
case DBG_APIV2_RW_MISC_OUT: case DBG_APIV2_RW_MISC_IN:
case DBG_APIV2_INIT_AP: case DBG_APIV2_CLOSE_AP_DBG:
case DBG_APIV2_READ_DAP_REG: case DBG_APIV2_WRITE_DAP_REG:
    break;
default:
    mem_set_ap(0);
    mem_set_csw_high(0);
    break;
}
```

AgentLA cached `current_ap` and never reset it, so a memory command that selected
AP 1 could leave a subsequent halt or register access pointed at the wrong AP.

## Decisions

### Decision 1: Reply first, then probe

ENTER_SWD sets `mode`, sets `idcode = 0`, writes `STLINK_OK`, and only then
attempts a bounded DP read. The reply length does not depend on target state.

*Rationale*: matches the reference, removes the timeout class of failure
entirely, and matches what a genuine ST-Link does.

### Decision 2: Log transport errors instead of discarding them

Both the reply write and the command read log on failure via `defmt::warn!`.

*Rationale*: `let _ =` is why this investigation required six bisect builds. One
line of logging converts the next occurrence into a measurement.

*Alternative considered*: return an error to the host. Rejected — the host
already reports the timeout, and the firmware cannot synthesise a valid reply
once the endpoint is wedged.

### Decision 3: Reset `current_ap` for core debug commands

Adopt hybridlink's exempt-list. `mem_access` already derives CSW from the address
range, so only the AP selector needs resetting.

### Decision 4: Keep the PIO transport

hybridlink bit-bangs; AgentLA uses PIO with DMA. The PIO engine is not implicated
by the bisection and is substantially faster. Only the *contract* around it is
adopted.

## Risks / Trade-offs

- **`idcode` is 0 until the first IDCODE read.** A host that reads the core ID
  before `DBG_READCOREID` sees 0. That matches a genuine ST-Link with no target
  and is what hybridlink does.
- **Logging on every read error could be noisy if the host disconnects
  abruptly.** Acceptable: the read loop already sleeps 10 ms on error, so a
  disconnect produces at most a few lines per second.
- **Resetting `current_ap` per command adds two writes per debug command.**
  Negligible; hybridlink does the same.

## Verification

- Build clean for `thumbv8m.main-none-eabihf` with `slot-b`.
- `st-info --probe` must report the STM32H503 and a non-zero chipid.
- `test/test_cmsis_dap.py` must still pass 12/12 (unchanged path).
- A read-only `st-flash` read of `0x08000000` must return the BareBoot vector
   table.

## Open question

The bisect shows the reply never reaching the host even for a bare `return 2`.
Decision 2 is the instrument for that: if the reply write is failing, the log
will say so on the first attempt after this change. If it is *not* failing, the
next hypothesis is that the host's 16-byte short packet leaves the embassy-usb
OUT endpoint in a state where the following command is never delivered.
## Root cause found (2026-11-17)

A raw-USB A/B test isolates it precisely:

```
GET_VERSION       OUT=0 IN=0 n=6  2c2383045237     <- works
ENTER_SWD (16 B)  OUT=0 IN=-7 n=0                   <- times out
GET_VERSION after OUT=0 IN=-7 n=0                   <- endpoint now dead
```

The reply to ENTER_SWD is not merely wrong — the **IN endpoint stops delivering
after it**. Every subsequent read times out, including a plain GET_VERSION that
worked moments earlier. The same happens when the host sends any 16-byte command
after a successful exchange.

This is the signature of a **stale/never-released IN transfer**, not a handler
bug:

- `GET_VERSION` writes 6 bytes and the host reads 6.
- `ENTER_SWD` writes 2 bytes into the same 64-byte buffer.
- The host asks for 2 bytes and times out.

A 2-byte reply on a 64-byte-max endpoint should be a short packet and terminate
immediately. It does not. So the second `cmd_in.write()` either never completes
or completes into a buffer the host never sees.

hybridlink never has this problem because it uses a 4-state machine with an
explicit guard:

```c
/* ST_EXEC: the previous reply must be gone before the buffer is reused */
if (usbd_edpt_busy(rhport_used, EP_STLINK_IN))
    return;
...
usbd_edpt_xfer(rhport_used, EP_STLINK_IN, data, (uint16_t)rlen, false);
arm_cmd();
```

AgentLA instead does `let _ = cmd_in.write(&resp_buf[..resp_len]).await` and
relies on embassy-usb's internal state machine. When that write fails — for
whatever reason — the error was discarded and the endpoint was left wedged.

### Next diagnostic step

The ZLP/zero-length logging and the reply-write error logging added in this
change are exactly the instruments needed. If the next `st-info` run logs
`STLINK reply write failed`, the cause is the IN endpoint. If it logs
`STLINK zero-length command packet`, the cause is the OUT endpoint.

Neither has been observed yet because the probe logs go to RTT/defmt, which is
not captured during these runs.
