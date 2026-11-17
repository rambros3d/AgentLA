## 1. Gate the HTTP routes

- [x] 1.1 Add `const HTTP_FLASH_ENABLED: bool = false;` in `firmware-rs/src/rest.rs`.
- [x] 1.2 Add a shared `http_flash_disabled_response()` helper that writes the
  `{"status":"disabled","error":"http_flash_disabled","use":"usb","modes":["cmsis","stlink"]}`
  body with HTTP 501.
- [x] 1.3 Gate `ProgrammerFlash` and `ProgrammerErase` before any programmer call.
- [x] 1.4 Gate `ProgrammerResetCatch`, `ProgrammerRescueStm32`, and `ProgrammerRescueNrf`.

## 2. Gate the MCP tools

- [x] 2.1 Gate `programmer_flash` in `firmware-rs/src/mcp.rs` with the same payload.
- [x] 2.2 Gate `programmer_unbrick` in `firmware-rs/src/mcp.rs` with the same payload.
- [x] 2.3 Confirm `crates/agentla-mcp` still lists the tools (schema unchanged); only the
  firmware handler returns the disabled result.

## 3. WebUI and documentation

- [x] 3.1 In `webui/agentla-bridge.js`, surface the `http_flash_disabled` message from
  `programmer.flash()`.
- [x] 3.2 Hide or annotate the flash/erase/unbrick buttons in `webui/app.js` when the disabled
  response is received.
- [x] 3.3 Update `README.md` and `docs/rest-api.md` to mark flash/erase/reset-catch/rescue as
  disabled and point to USB ST-Link / CMSIS-DAP.
- [x] 3.4 Update `skills/agentla-programmer/SKILL.md` to instruct agents to use the USB
  transport when `http_flash_disabled` is returned.

## 4. Tests

- [ ] 4.1 Update `test/test_bluepill_flash.py` and `test/flash_*.py` to expect 501 while the
  flag is off, or skip with a clear reason.
- [x] 4.2 Add a test that asserts the 501 body contains `http_flash_disabled` and `modes`.
- [x] 4.3 Add a test that `GET /api/v1/programmer/read` still returns 200 with the flag off.
- [ ] 4.4 Run `python3 -m pytest test/test_cmsis_dap.py` and confirm it still passes.

## 5. Firmware build and deployment

- [x] 5.1 Build release firmware (`cargo build --release -p firmware-rs`).
- [x] 5.2 Package and OTA to the probe, or flash Slot B over SWD.
- [x] 5.3 Confirm `POST /api/v1/programmer/flash` returns 501 with the disabled body.
- [x] 5.4 Confirm `GET /api/v1/programmer/read?addr=0x08000000` still returns 200.

## 6. USB transport verification (ref/stlink)

- [x] 6.1 Build `ref/stlink` with cmake (libusb 1.0 is present).
  Built `v1.9.0-21-g34de09c` at `/tmp/opencode/stlink-build` with
  `cmake -DCMAKE_BUILD_TYPE=Release` and `--target st-info st-flash`.
- [x] 6.2 Switch the probe to ST-Link mode:
  `curl -X POST "http://10.10.10.10/api/programmer/mode?mode=stlink"`
- [x] 6.3 Confirm USB enumeration changes to `0483:3752` (`lsusb`).
  `Bus 003 Device 019: ID 0483:3752 STMicroelectronics ST-LINK/V2.1`
- [ ] 6.4 Run `st-info --probe` and record the reported chip and flash size.
  BLOCKED: `Found 0 stlink programmers` (serial number too short, see 6b).
- [ ] 6.5 Run a read-only `st-flash` read of `0x08000000` and compare with the expected
  BareBoot vector table (`F87F0020 41020008`).
- [x] 6.6 Restore CMSIS-DAP mode:
  `curl -X POST "http://10.10.10.10/api/programmer/mode?mode=cmsis"`
  Restored; `lsusb` shows `1209:414d Generic Adi Linden` again.

### 6b. USB serial-number fix (blocking 6.4)

- [x] 6b.1 Replace the fixed `config.serial_number = Some("00000001")` in
  `firmware-rs/src/usb.rs` with a 24-hex-character serial derived from the RP2350 unique ID
  (plus a personality discriminator), matching `STLINK_SERIAL_LENGTH == 24`.
- [x] 6b.2 Confirm `lsusb -v -d 0483:3752 | grep iSerial` shows a 24-character value.
- [ ] 6b.3 Re-run 6.4-6.5.

## 7. Bookkeeping

- [x] 7.1 Run `openspec validate temporarily-disable-http-flash`.
- [ ] 7.2 Sync specs and archive the change once the flag is deployed.

---

## Findings

### 6.1-6.3 passed (2026-11-17)

`ref/stlink` v1.9.0-21-g34de09c builds with cmake and libusb 1.0.30. `st-info --version`
reports the expected version. Switching the probe to ST-Link mode re-enumerates USB as
`0483:3752 STMicroelectronics ST-LINK/V2.1`, and the network interface stays reachable.

### 6.4 failed: serial number too short (2026-11-17)

`st-info --probe` reports `Could not read the serial` and `Found 0 stlink programmers`.

Cause: `firmware-rs/src/usb.rs` sets an 8-character serial (`"00000001"`), but
`ref/stlink` requires `STLINK_SERIAL_LENGTH == 24` (a 96-bit UID as 24 hex digits). Raw USB
string-descriptor reads confirm the descriptor is present and correct; only its length is
rejected by `stlink_serial()`.

Tracked as 6b above. This is a firmware-side defect, not a probe-permissions issue.
### 6c. ST-Link ENTER_SWD timeout (new blocker, 2026-11-17)

After the serial fix, `st-info --probe` gets past enumeration and reports the probe, then fails:

```
ERROR usb.c: ENTER_SWD read reply failed: LIBUSB_ERROR_TIMEOUT
ERROR common_legacy.c: Failed to enter SWD mode
Found 1 stlink programmers
  version:    V2J48S35
  serial:     0007F5003752000100000000
```

- [ ] 6c.1 Determine why ENTER_SWD produces no 2-byte reply. Eight candidate causes were tried
  and reverted (see design.md "ST-Link transport investigation"): pin init on the bare
  `SwdTransport`, `assert_reset(false)`, `tristate_pins()` before claiming pins, `try_attach()`
  at task start, `ensure_initialized()` per transfer, driving the full ENTER_SWD sequence from
  the USB task, and a version-word re-check. None changed the outcome.
- [ ] 6c.2 Check whether `StLinkHandler` needs `ensure_initialized()` on its transfer path, as
  `CmsisDapHandler` does at `dap.rs` (eight call sites).
- [ ] 6c.3 Check whether `SwdTransport` survives a personality switch without a full
  `deinit()` / `init_pins()` cycle.
- [ ] 6c.4 Until 6c.1 lands, ST-Link mode must be treated as non-functional and CMSIS-DAP as the
  only verified USB transport.
