## Why

`st-info --probe` sees the probe but fails to enter debug mode:

```
Found 1 stlink programmers
  version:    V2J48S35
  serial:     C000DDC77247F50037520001
  flash:      0 (pagesize: 0)
  chipid:     0x000
ERROR usb.c: ENTER_SWD read reply failed: LIBUSB_ERROR_TIMEOUT
ERROR common_legacy.c: Failed to enter SWD mode
```

Bisection against raw USB endpoints narrowed the failure:

| Test | Result |
|---|---|
| `GET_VERSION` (0xF1) | works, replies 6 bytes |
| `GET_TARGET_VOLTAGE` (0xF7) | works, replies 8 bytes |
| `GET_CURRENT_MODE` (0xF5) | works, replies 2 bytes |
| `DBG_READCOREID` (0xF2 0x22) | works, replies 4 bytes |
| `ENTER_SWD` (0xF2 0x30 0xA3) | **times out, no reply** |

A bisect build with `ENTER_SWD` reduced to a bare `return 2` (no PIO, no logic)
still timed out, while `DBG_READCOREID` from the same `debug_command` function
replied normally. So the defect is not in the SWD engine or the handler logic.

Comparing against `ref/hybridlink/app/stlink.c` and `app/stlink_usb.c` shows
four structural differences that AgentLA gets wrong.

## What Changes

- **Reply before target access.** hybridlink's ENTER_SWD handler
  (`app/stlink.c:339-351`) reports `STLINK_OK` unconditionally and comments:
  *"a genuine ST-Link reports success here even without a target; the following
  IDCODE read tells the host whether one is present."* AgentLA instead ran a full
  DP probe before replying, which can take seconds and overruns the host's 3 s
  command timeout.
- **Log reply-write failures.** The ST-Link USB task discarded every
  `cmd_in.write()` error with `let _ =`. A failed reply write is the difference
  between "the host timed out" and a diagnosable endpoint error.
- **Log command-read failures.** Same treatment for the OUT endpoint read.
- **Reset AP/CSW for core debug commands.** hybridlink's `debug_command` prologue
  (`app/stlink.c:321-333`) forces AP 0 and the default CSW for every non-DAP
  command. AgentLA cached `current_ap`, so a prior memory command could leave a
  stale AP selected for a subsequent halt or register access.

## Capabilities

### New Capabilities
<!-- None -->

### Modified Capabilities
- `domain-crate-programmer`: Add the ST-Link command-path contract: reply before
  target access, reset AP/CSW for core debug commands, surface transport errors.
- `cmsis-dap`: ST-Link personality interoperability requirements.

## Impact

- `crates/agentla-programmer/src/stlink.rs`: ENTER_SWD reply ordering, AP/CSW
  reset prologue.
- `firmware-rs/src/usb.rs`: error logging on the ST-Link reply write and
  command read.
- `test/`: host unit tests for the reply-before-probe contract.
- Does not change the CMSIS-DAP or WCH-Link personalities.
- Does not change the PIO transport or flash algorithms.

## Out of Scope

- `BKSEL` dual-bank geometry in `flash-algorithms` (separate change).
- `swd-turnaround-timing` spec accuracy (separate change).
- The 7.2 s N20-Servo boot scan (separate repo).