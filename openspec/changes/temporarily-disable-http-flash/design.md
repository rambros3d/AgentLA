## Context

`firmware-rs` exposes two programming surfaces:

```
USB  -> ProgrammerPersonality { CmsisDap, StLink, WchLink }
        CMSIS-DAP / ST-Link : transport only, host owns the algorithm
        WCH-Link            : native CH32 flash (protocol requires it)

HTTP -> /api/v1/programmer/{flash,erase,reset-catch,rescue/*}
        native direct engines only (STM32H5/F1 paths)
```

The USB surface is the mature one. The HTTP surface carries the vector-catch trap and the
three-implementation flash drift. Disabling the HTTP surface is a containment action, not a
refactor: it changes which door is open, not how the room is built.

## Goals / Non-Goals

**Goals:**
- Make the vector-catch trap unreachable through the default HTTP API.
- Keep the change reversible in one line.
- Preserve every non-flash diagnostic route.
- Keep the test suite green with the flag off.
- Establish USB ST-Link / CMSIS-DAP as the documented programming path.

**Non-Goals:**
- Fixing the flash engines, the bank geometry, or the session teardown.
- Removing the flash code or the routes.
- Changing the USB personalities or the PIO transport.
- Changing the WCH-Link native flash path (protocol requirement, and unrelated to the trap).

## Decisions

### Decision 1: One build-time flag, not five guards

```rust
// firmware-rs/src/rest.rs
const HTTP_FLASH_ENABLED: bool = false;
```

Each handler checks it once at the top and returns the same disabled response. A single constant
is the re-enable switch; no route table changes, no feature plumbing.

*Alternative considered*: remove the routes entirely. Rejected because clients would get a bare
404, the OpenSpec capability would need a removal delta, and re-enabling would be a larger diff.

### Decision 2: `501 Not Implemented` with a machine-readable body

```json
{"status":"disabled","error":"http_flash_disabled","use":"usb","modes":["cmsis","stlink"]}
```

501 says "this server does not implement this" without implying a transient fault (503) or a bad
request (400). The `use` and `modes` fields tell a client or an agent what to do instead, which
matters because the MCP skills currently instruct agents to call `programmer_flash`.

### Decision 3: Disable `reset-catch` and the rescue routes too

`POST /api/v1/programmer/reset-catch` calls `CortexM::connect_under_reset`, which writes
`DEMCR = 0x0100_0001` and never clears `VC_CORERESET`. With flash disabled it is still a way to
leave the target trapped: the core halts at cycle 0, the call returns, and the next reset
re-halts. The rescue routes perform mass erase, which is flash. All five belong in the disabled
set until the session-teardown change lands.

### Decision 4: Keep the read-only routes

```
GET  /api/v1/programmer/target          keep
GET  /api/v1/programmer/read            keep (non-intrusive AHB-AP read)
POST /api/v1/programmer/reset           keep (nRST/BOOT0 only, no flash)
GET  /api/v1/programmer/rtt/*           keep
GET  /api/v1/programmer/swo/status      keep
GET/POST /api/programmer/mode           keep (this is how you reach ST-Link)
GET/POST /api/programmer/stlink/version keep
```

`reset` is safe because it only drives pins. `target` and `read` are the diagnostics that make
the USB path usable from the WebUI.

### Decision 5: Gate MCP tools with the same error, not a different one

`programmer_flash` and `programmer_unbrick` return the same `http_flash_disabled` payload. The
agent-facing skills then fail with an actionable message instead of an opaque `TargetError`.

### Decision 6: Leave the WebUI buttons in place, show the disabled state

Hiding the buttons requires a capability negotiation that does not exist yet. Showing the 501
message is a smaller change, keeps the UI honest, and automatically explains itself when the flag
is flipped back.

## Risks / Trade-offs

- **A stale client may treat 501 as a bug.** Mitigated by the `use`/`modes` fields and the docs.
- **The trap is still reachable if someone calls `connect_under_reset` from another path.** Today
  the only HTTP path is `reset-catch`, which is gated. The GDB server does not call it.
- **Tests that assert successful HTTP flashing will fail.** They are updated or skipped in the
  same change; this is the intended signal that the capability is off.
- **Re-enabling requires a firmware rebuild and OTA.** That is deliberate: the flag is a build
  decision, not a runtime toggle a client can flip.

## Verification Plan

1. `cargo build --release -p firmware-rs` and OTA to the probe.
2. `POST /api/v1/programmer/flash` returns 501 with `http_flash_disabled`.
3. `POST /api/v1/programmer/erase` returns 501.
4. `GET /api/v1/programmer/read?addr=0x08000000` still returns 200 with flash bytes.
5. `GET /api/v1/programmer/target` still identifies the STM32H503.
6. `POST /api/programmer/mode?mode=stlink` re-enumerates USB as `0483:3752`.
7. `st-info --probe` from `ref/stlink` reports the H503.
8. `st-flash` read (no write) returns the expected vector table.

Step 8 is deliberately read-only. Any erase or program is a separate, explicitly approved step.

## Bench Verification (2026-11-17)

Executed against the live probe. `ref/stlink` builds cleanly here
(`cmake -B build -DCMAKE_BUILD_TYPE=Release`, `cmake --build --target st-info st-flash`,
libusb 1.0.30). `st-info --version` reports `v1.9.0-21-g34de09c`.

### Result: ST-Link mode enumerates but `st-info` cannot open the probe

```
$ curl -X POST "http://10.10.10.10/api/programmer/mode?mode=stlink"
{"status":"ok","mode":"stlink","re-enumerating":true}

$ lsusb
Bus 003 Device 019: ID 0483:3752 STMicroelectronics ST-LINK/V2.1     <- correct VID/PID

$ st-info --probe
2026-11-17T13:34:21 WARN usb.c: Could not read the serial of device 003:019
2026-11-17T13:34:21 ERROR usb.c: Failed to open device 003:019
Found 0 stlink programmers
```

### Root cause: the serial number is too short

`firmware-rs/src/usb.rs:434` sets a fixed 8-character serial:

```rust
config.serial_number = Some("00000001");
```

`ref/stlink/inc/stlink.h:92` requires 24 hex characters (a 96-bit UID):

```c
#define STLINK_SERIAL_LENGTH                 24 ///< characters of the serial number
```

`ref/stlink/src/stlink-lib/usb.c:1188-1222` accepts exactly two serial encodings:

```c
if(len == ((STLINK_SERIAL_LENGTH + 1) * 2)) {          /* len == 50: 24 chars + NUL  */
    ret = libusb_get_string_descriptor_ascii(...);
} else if(len == ((STLINK_SERIAL_LENGTH / 2 + 1) * 2)) { /* len == 26: 12 raw bytes  */
    ...
} else {
    return 0;                                          /* <- 8-char serial lands here */
}
```

Verified directly over USB control transfers:

```
string descriptor 0  -> [4, 3, 9, 4]   (langid 0x0409, fine)
string descriptor 3  -> "00000001"     (8 chars, descriptor length 18 = 2 + 16)
```

`stlink_serial()` therefore returns 0, the device is rejected during both the enumeration loop
(`usb.c:1292`) and the post-open check (`usb.c:1333`), and `st-info` reports zero programmers.

This is not a permissions problem. `/dev/bus/usb/003/019` is `crw-rw-rw-`, the OpenOCD udev rule
covers `0483:3752`, and raw string-descriptor reads succeed from userspace.

### Required fix (one line, in the firmware)

Use the RP2350's unique 96-bit ID to build a 24-hex-character serial, for example:

```
PICO2 UID (8 bytes) -> 16 hex chars
+ 8 hex chars of a board/personality discriminator
= 24 chars, matching STLINK_SERIAL_LENGTH
```

A fixed 24-character serial would also satisfy the length check, but would collide if more than
one probe is attached. Deriving it from the unique ID matches how a real ST-LINK behaves
(README: "serial = the 96-bit UID as 24 hex digits").

### Secondary finding: the probe still serves HTTP while in ST-Link mode

`GET http://10.10.10.10/api/programmer/mode` continued to answer while USB was enumerated as
`0483:3752`. The network stack is independent of the USB personality, which is convenient for
recovery but means the ST-Link mode does not actually "hand over" the device. Not a defect, but
worth documenting so an operator does not assume ST-Link mode implies exclusive USB control.

### Next verification step

After the serial fix and a rebuild/OTA:

1. `st-info --probe` must report `ST-LINK/V2.1 (no mass storage)`.
2. `st-info --chipid` must report the H503 chip id.
3. A read-only `st-flash` read of `0x08000000` must return `F8 7F 00 20 41 02 00 08`.
4. `st-flash` erase/program remains a separate, explicitly approved step.
## ST-Link transport investigation (2026-11-17)

After the serial-number fix, `st-info` sees the probe but times out on ENTER_SWD. Eight
hypotheses were tried against the live probe. Each was built, OTA'd, and tested with
`st-info --probe`; none changed the outcome.

| # | Hypothesis | Result |
|---|---|---|
| 1 | `SwdTransport::new()` leaves `initialized = false`, so ENTER_SWD drives unconfigured GPIOs | No change |
| 2 | nRST is left asserted by a previous personality | No change |
| 3 | `tristate_pins()` is needed before claiming the SWD pins | No change |
| 4 | The DP needs `try_attach()` before the host's sequence | No change |
| 5 | `ensure_initialized()` must run per transfer (as in `dap.rs`) | No change |
| 6 | The USB task should drive the full ENTER_SWD sequence itself | No change |
| 7 | The `GET_VERSION` word `0x2C23` is parsed wrong by the host | Ruled out - host reads `V2J48S35` correctly |
| 8 | `stm32h5` AP1 is needed for ST-Link to see the H503 | Not reached |

Findings worth keeping:

- **The serial fix is real and independent.** `st-info` went from `Found 0` to `Found 1` with
  `serial: 0007F5003752000100000000` and `version: V2J48S35`.
- **GET_VERSION works.** The version word `0x2C23` encodes as `b0=0x2C, b1=0x23` and the host
  decodes `stlink_v=2, jtag_v=48, swim_v=35` correctly.
- **GET_TARGET_VOLTAGE sometimes fails** with `LIBUSB_ERROR_NO_DEVICE` / `_IO` after the
  ENTER_SWD failure, which is downstream of the dead debug link, not a separate bug.
- **The ST-Link handler has no `ensure_initialized()` call.** `CmsisDapHandler` calls it at eight
  sites in `dap.rs`. This is the strongest untested hypothesis, because without it the PIO state
  machine is only ever initialized by the one-time setup at task start.

The likely shape of the real bug: `SwdTransport` is a bit-level PIO engine that assumes the
caller has already established the SWD line state, and `StLinkHandler` was written against a
different transport contract than the one `firmware-rs` actually provides. Fixing it is a
separate change with a proper reproduction, not another blind build.

Until then: **CMSIS-DAP is the only verified USB transport.**
