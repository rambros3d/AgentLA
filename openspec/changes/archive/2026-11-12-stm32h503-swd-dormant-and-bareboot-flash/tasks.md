## 1. CoreSight DP v2 CSW Configuration & Reset Catch Engine

- [x] 1.1 In `crates/agentla-programmer/src/swd.rs`, update `ADIV5_AP_CSW_DEFAULT` to `0x23000012` (`HNONSEC = 0`)
- [x] 1.2 In `crates/agentla-programmer/src/target/cortex_m.rs`, add `csw: u32` to `CortexM`, and update `mem_read32` and `mem_write32` to use `self.csw`
- [x] 1.3 In `crates/agentla-programmer/src/target/cortex_m.rs`, implement tight burst write of DHCSR (`0xA05F0003`) and DEMCR (`0x01000001` with `VC_CORERESET`) immediately after `self.swd.assert_reset(false)` in `connect_under_reset()`
- [x] 1.4 In `firmware-rs/src/programmer/target/mod.rs`, ensure AP 1 (AHB5-AP) and `csw = 0x23000012` are selected when `dpidr == 0x6BA02477` or `0x0BE12477` is detected in `detect_target_port()`

## 2. Firmware Build & True A/B Dual-Slot OTA Update to Slot A

- [x] 2.1 Compile `firmware-rs` release binary for `pico2` target targeting Slot A (`ORIGIN = 0x10004000`, without `--features slot-b`)
- [x] 2.2 Package `.ota.bin` image using `tools/package_ota.py` for Slot A
- [x] 2.3 Deploy `.ota.bin` to AgentLA via `POST http://10.10.10.10/api/v1/firmware/ota`
- [x] 2.4 Verify AgentLA reboots cleanly into Slot A and reports `status: "healthy"` and active slot A

## 3. Hardware Halt Under Reset Verification

- [x] 3.1 Send `POST http://10.10.10.10/api/v1/programmer/reset-catch` and verify returned target state
- [x] 3.2 Verify target recognition identifies "STM32H503 Microcontroller (Cortex-M33)" (`0x474`) via `probe_target` / `target_diagnose`

## 4. BareBoot I3C Bootloader Flashing

- [x] 4.1 Verify the STM32H503 I3C bootloader binary from `/home/sun/Filelink/Firmwares/bareBoot/bareboot-stm32h503-i3c.bin` (3,216 bytes)
- [x] 4.2 Erase Sector 0 and program bootloader binary to STM32H503 flash base `0x08000000`
- [x] 4.3 Release target reset and verify BareBoot PA0 Blue LED blinks at 5 Hz

## 5. Host Bus Verification & N20-Servo Application Deployment

- [x] 5.1 Query `/api/v1/bridge/i2c/scan` over Pico GP16/GP17 to verify response from static address `0x40`
- [x] 5.2 Relocate `N20-Servo/firmware/memory.x` FLASH origin to `0x08002000` with length `120K` (122,880 bytes)
- [x] 5.3 Compile `N20-Servo/firmware` application binary and deploy over the BareBoot protocol via I2C/I3C bus
