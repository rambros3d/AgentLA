## Why

The STM32H503 (Arm Cortex-M33 with CoreSight DP v2) fails to attach over SWD during normal boot because existing flash firmware reconfigures SWD pins (PA13/PA14) or enters deep low-power STOP mode within 300 microseconds of reset deassertion. Host-PC debug scripts cannot halt the target in time due to USB Full-Speed frame scheduling latency (>= 1 millisecond). Furthermore, existing probe firmware failed connect-under-reset due to:
1. Hardcoded Non-Secure CSW (`0xE3000012` with `HNONSEC = 1`) triggering AHB bus faults on Private Peripheral Bus accesses in Secure Cortex-M33 state.
2. Ineffective DHCSR/DEMCR writes executed only while reset was asserted low (when core logic was held in reset) and missing the tight post-deassertion `t_RSTTEMPO` vector catch window.

To enable handsfree flashing of the BareBoot I3C bootloader and N20 Servo firmware, AgentLA RP2350 local firmware must execute tight-burst vector catch under hardware reset (`nRST` on GP6) with Secure CSW (`0x23000012`), program BareBoot to Sector 0 (`0x08000000`), and verify host bus communication at static address `0x40`.

## What Changes

- Set ADIv5 / ADIv6 CSW to Secure privileged transfer mode (`0x23000012`, `HNONSEC = 0`) for Cortex-M33 debug and make CSW configurable in `CortexM`.
- Implement tight-burst write of `DHCSR` (`0xA05F0003`) and `DEMCR` (`0x01000001`) across `nRST` rising edge in `CortexM::connect_under_reset()`.
- Ensure AP 1 (AHB-AP) and Secure CSW are active during target detection (`detect_target_port()`) when CoreSight DP v2 is detected (`dpidr == 0x6BA02477` / `0x0BE12477`).
- Build updated `firmware-rs` for Slot A (`ORIGIN = 0x10004000`) and deploy via True A/B dual-slot OTA (switching from active Slot B).
- Execute `POST /api/v1/programmer/reset-catch` to halt the target at the reset vector before user code disables SWD.
- Program BareBoot I3C bootloader (`bareboot-stm32h503-i3c.bin`, 3,216 bytes) into STM32H503 Flash Sector 0 (`0x08000000`).
- Verify bootloader execution via PA0 Blue LED (5 Hz blink) and I2C/I3C bus scan at static address `0x40`.
- Relocate N20-Servo firmware application to `0x08002000` (120 KB), compile, and flash over the BareBoot protocol.

## Capabilities

### New Capabilities
- `bareboot-i3c-bootloader-flashing`: End-to-end configuration, packaging, flashing, and verification of the I3C/I2C BareBoot bootloader and relocated N20-Servo application.

### Modified Capabilities
- `cortex-m33-reset-catch`: Synchronous nanosecond-scale hardware reset deassertion and burst vector catch latching on RP2350 local firmware with Secure CSW.
- `target-detection`: Auto-select AP 1 (AHB-AP) and Secure CSW (`0x23000012`) on CoreSight DP v2 Cortex-M33 targets to read DBGMCU IDCODE at `0x44022000`.

## Impact

- Affects `crates/agentla-programmer` (`swd.rs`, `cortex_m.rs`).
- Affects `firmware-rs` (`programmer/target/mod.rs`, `rest.rs`).
- Probe firmware updated safely via `/api/v1/system/firmware/ota` into Slot A without touching LittleFS user storage partitions (`0x00204000+`).
