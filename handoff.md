# AgentLA to STM32H503 SWD & Bootloader Testing Handoff

## 1. Executive Summary & Objective

- **Target Board**: N20 Servo Motor Controller Board featuring the STMicroelectronics STM32H503KBU6 microcontroller (ARM Cortex-M33 with TrustZone).
- **Programmer / Probe**: AgentLA running on Raspberry Pi Pico 2 (RP2350, Dual Hazard3 / Cortex-M33), active on Slot B (`10.10.10.10`) with CMSIS-DAP v1 (HID) and CMSIS-DAP v2 (USB Bulk).
- **Core Goals**:
  1. Establish stable debug control (SWD / CMSIS-DAP) over the STM32H503 under reset without hardware BOOT0 jumpers.
  2. Flash the BareBoot I3C bootloader image (`bareboot-stm32h503-i3c.bin`, 3,216 bytes) into STM32H503 Flash Sector 0 (`0x08000000`).
  3. Verify host bus communication (I3C/I2C static address `0x40`).
  4. Build, relocate (`0x08002000`), and flash the N20-Servo firmware application over the I3C/I2C BareBoot bootloader.

---

## 2. Hardware Wiring Configuration

| Signal | Pico 2 (AgentLA) Pin | STM32H503 (N20 Servo) Pin | Function |
| :--- | :--- | :--- | :--- |
| **SWCLK** | GP4 (Pin 6) | PA14 (SWCLK) | SWD Clock |
| **SWDIO** | GP5 (Pin 7) | PA13 (SWDIO) | SWD Bi-directional Data |
| **nRST** | GP6 (Pin 9) | NRST (Reset) | Hardware Target Reset (Active Low) |
| **I2C/I3C SDA** | GP16 (Pin 21) | PB4 (I3C1_SDA) | Telemetry & Bootloader Comm |
| **I2C/I3C SCL** | GP17 (Pin 22) | PB3 (I3C1_SCL) | Telemetry & Bootloader Comm |
| **GND** | Pin 3, 8, 13, 18, 23 | GND | Common Reference Ground |
| **3V3** | 3V3 OUT (Pin 36) | 3V3 | System Logic Power (3.3V) |

---

## 3. Progress Completed

### 3.1 AgentLA Firmware & Dual-Slot A/B OTA
- Diagnosed and fixed the hardcoded `apsel = 0` defect in [`crates/agentla-programmer/src/pio/swd.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/crates/agentla-programmer/src/pio/swd.rs#L557) and [`crates/agentla-programmer/src/swd.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/crates/agentla-programmer/src/swd.rs#L161): dynamic AP selection `(self.ap_select >> 24) as u8` was implemented.
- Compiled Slot A (`ORIGIN = 0x10004000`) and Slot B (`ORIGIN = 0x10104000`, `--features slot-b`) firmware binaries.
- Successfully performed OTA upgrade using [`tools/package_ota.py`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/tools/package_ota.py). AgentLA is live and verified on Slot B.

### 3.2 CMSIS-DAP Protocol Verification
- Verified CMSIS-DAP over USB HID (`/dev/hidraw5`) and CMSIS-DAP v2 over USB Bulk (`Interface 3`).
- Confirmed DP and AP enumeration:
  - **DPIDR**: `0x6BA02477` (ARM CoreSight SWD-DP v2, Designer `0x23B`, Version `0x2`).
  - **AP 0 (APB-AP)**: IDR `0x54770002` (CoreSight APB-AP).
  - **AP 1 (AHB-AP)**: IDR `0x84770001` (CoreSight AHB5-AP).
  - **TARGETID**: `0x14740041` (Part number `0x474` = STM32H503, Designer `0x041` = STMicroelectronics).

### 3.3 Bootloader Artifacts Ready
- BareBoot I3C bootloader built at [`/home/sun/Filelink/Firmwares/bareBoot/bareboot-stm32h503-i3c.bin`](file:///home/sun/Filelink/Firmwares/bareBoot/bareboot-stm32h503-i3c.bin) (3,216 bytes). Fits into STM32H5 Sector 0 (`0x08000000` to `0x08002000`).
- N20 Servo application ready at [`/home/sun/Filelink/Firmwares/N20-Servo/firmware`](file:///home/sun/Filelink/Firmwares/N20-Servo/firmware).

---

## 4. Root Cause Analysis & Technical Findings

### 4.1 Target SWD Disconnect on Normal Boot
- The pre-existing firmware loaded in STM32H503 Flash boots in less than 300 microseconds after reset deassertion.
- Upon booting, the flash firmware executes code that immediately reconfigures `PA13`/`PA14` as GPIOs or enters low-power STOP mode, terminating SWD clock/data response.
- As a result, standard attach fails unless captured strictly under reset.

### 4.2 Why Host-PC Python Scripts Cannot Catch Reset
- When a Python script on the host PC releases reset via `DAP_SWJ_Pins` (`0x10`) and subsequently sends `DAP_Transfer` (`0x05`) to write `DHCSR` and `DEMCR`:
  - Each USB packet roundtrip over USB Full-Speed is subject to host frame scheduling latency (minimum 1,000 microseconds / 1 millisecond).
  - The STM32H503 reset delay tempo (`t_RSTTEMPO`) is only ~200 to 400 microseconds.
  - The target boots and kills SWD before the second USB frame arrives from the host PC.
- **Solution**: The reset release and vector catch latching must happen directly within the AgentLA RP2350 local firmware where register transitions take sub-microsecond intervals (nanosecond GPIO toggle + 1 microsecond SWD write).

### 4.3 Why Existing `connect_under_reset` Timed Out in AgentLA Firmware
1. **Secure CSW Requirement**:
   - In [`crates/agentla-programmer/src/swd.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/crates/agentla-programmer/src/swd.rs#L23), `ADIV5_AP_CSW_DEFAULT` is hardcoded to `0xE3000012`.
   - Bit 30 of CSW is `HNONSEC` (1 = Non-Secure, 0 = Secure). In `0xE3000012`, bit 30 is set to `1`.
   - On reset, STM32H5 Cortex-M33 starts in **Secure state**. Any transfer to Private Peripheral Bus (PPB `0xE000ED00`..`0xE000EDFC`) with `HNONSEC = 1` triggers an immediate bus FAULT (`ack=4`).
   - CSW must be configured to `0x23000012` (or `0x23000002`) with `HNONSEC = 0` for Cortex-M33 secure debug.
2. **Timing of DHCSR/DEMCR Writes During Reset Pulse**:
   - In [`crates/agentla-programmer/src/target/cortex_m.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/crates/agentla-programmer/src/target/cortex_m.rs#L185), `connect_under_reset` attempts to write `DHCSR` and `DEMCR` while `nRST` is held low at 0V. Because the core logic is in reset, the AHB bus transfer does not latch.
   - When `self.swd.assert_reset(false)` is subsequently called, the loop only executes `mem_read32(DHCSR)` without re-writing `DHCSR_C_DEBUGEN | DHCSR_C_HALT` or `DEMCR_VC_CORERESET` in the immediate microsecond window following deassertion.
   - Adding a tight burst write of `DHCSR` and `DEMCR` right across the `nRST` rising edge ensures vector catch captures the core at the reset vector before the first instruction fetches.

---

## 5. Action Items for Resuming Work

1. **Update [`crates/agentla-programmer/src/target/cortex_m.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/crates/agentla-programmer/src/target/cortex_m.rs)**:
   - Add a configurable `csw: u32` field to `CortexM` struct (defaults to `0x23000012` for Secure Cortex-M33, or auto-selected when `dpidr == 0x6BA02477`).
   - In `mem_read32` and `mem_write32`, write `self.csw` instead of hardcoded `ADIV5_AP_CSW_DEFAULT`.
   - In `connect_under_reset`, insert an immediate burst write of `DHCSR` (`0xA05F0003`) and `DEMCR` (`0x01000001`) right after `self.swd.assert_reset(false)`.
2. **Build & Flash Updated AgentLA Slot A Firmware**:
   - Build `firmware-rs` for Slot A (`ORIGIN = 0x10004000`), package with `package_ota.py`, and upload via `http://10.10.10.10/api/v1/firmware/ota`.
3. **Execute Software Halt Under Reset**:
   - Run `POST http://10.10.10.10/api/v1/programmer/reset-catch` and verify returned `{"status":"ok","halted":true,"pc":...}`.
4. **Flash BareBoot Image**:
   - Erase Sector 0 and program `bareboot-stm32h503-i3c.bin` (3,216 bytes) at `0x08000000`.
   - Release reset and confirm Blue LED on `PA0` blinks at 5 Hz.
5. **Verify Host Bus & Deploy Application**:
   - Query `/api/v1/bridge/i2c/scan` over Pico GP16/GP17 to verify response from static address `0x40`.
   - Relocate `N20-Servo/firmware/memory.x` to `0x08002000` (length 120 KB), compile, and flash using the BareBoot protocol over I3C/I2C.
