## Why

DM (Debug Module) registers in the RISC-V DTM are inaccessible via SWD AP access, while SBA (System Bus Access) DMI registers work fine. This blocks `riscv_halt()`, which in turn blocks all flash erase/write operations. The root cause is likely a DP_SELECT bank mapping or DMI addressing issue specific to the WCH CH32V203's non-standard SWD DTM implementation.

## What Changes

- Add diagnostic instrumentation to identify which SWD AP access patterns succeed vs fail for DM registers
- Fix the DMI register access mechanism so DM registers (0x10 DMCONTROL, 0x11 DMSTATUS) are accessible
- Restore full halt/resume/flash-erase/write capability for CH32V203 via SWD
- Remove diagnostic code once root cause is confirmed and fixed

## Capabilities

### New Capabilities

- `riscv-dmi-dm-access`: Fix Debug Module register access via SWD DTM so halt, resume, abstract commands, and flash operations work reliably on CH32V203

### Modified Capabilities

- `swd-engine`: May need DP power-up or bank selection fixes in swd_write_ap/swd_read_ap for WCH DTM compatibility

## Impact

- Firmware: `agentla_riscv_dmi.c` — DMI read/write functions, halt/resume
- Firmware: `agentla_swd.c` — potential DP power-up or AP bank selection changes
- Test: `test/test_ch32v203_swd_blink.py` — depends on halt working for flash operations
- Blocking: All flash erase/write/readback operations require working DM access
