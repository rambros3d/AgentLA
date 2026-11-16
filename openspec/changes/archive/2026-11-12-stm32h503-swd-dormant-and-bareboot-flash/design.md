## Context

The target system is an N20 Servo Motor Controller Board equipped with an STMicroelectronics STM32H503KBU6 microcontroller (Arm Cortex-M33 with ARM Debug Interface v6 / ADIv6, CoreSight Debug Port v2 / DPv2). 

Hardware wiring to AgentLA on Raspberry Pi Pico 2:
- SWCLK: GP4 (Pin 6) -> STM32H503 PA14
- SWDIO: GP5 (Pin 7) -> STM32H503 PA13
- nRST: GP6 (Pin 9) -> STM32H503 NRST
- I2C/I3C SDA: GP16 (Pin 21) -> STM32H503 PB4 (I3C1_SDA)
- I2C/I3C SCL: GP17 (Pin 22) -> STM32H503 PB3 (I3C1_SCL)
- GND & 3V3 Logic Supply

Pre-existing firmware in the STM32H503 boots in < 300 µs after reset deassertion and immediately reconfigures SWD pins PA13/PA14 or enters low-power STOP mode, terminating SWD responses. Host-PC Python scripts over USB Full-Speed cannot halt the target in time because USB frame scheduling latency (>= 1,000 µs) far exceeds the STM32H5 reset tempo (`t_RSTTEMPO` ~200 to 400 µs). Halting and vector catching must therefore happen synchronously within AgentLA firmware running directly on the RP2350 microcontroller.

Furthermore, previous attempts to run `connect_under_reset` from probe firmware timed out due to two defects:
1. `ADIV5_AP_CSW_DEFAULT` had `0xE3000012` where Bit 30 (`HNONSEC`) was set to 1. On Cortex-M33 in Secure state, accesses to the Private Peripheral Bus (`0xE000ED00`..`0xE000EDFC`) with `HNONSEC = 1` trigger immediate bus faults (`ack = 4`).
2. DHCSR and DEMCR writes were executed only while `nRST` was held low at 0V (when core logic was in reset and bus transfers failed or were ignored), with no immediate burst write across the reset rising edge.

## Goals / Non-Goals

**Goals:**
- Make CSW configurable in `CortexM` and set default to Secure privileged transfer mode (`0x23000012`, `HNONSEC = 0`).
- Implement tight-burst write of DHCSR (`0xA05F0003`) and DEMCR (`0x01000001` with `VC_CORERESET`) immediately after deasserting `nRST` in `connect_under_reset()`.
- Ensure AP 1 (AHB5-AP) and Secure CSW are selected during target detection (`detect_target_port()`) when `dpidr == 0x6BA02477` / `0x0BE12477`.
- Build `firmware-rs` for Slot A (`ORIGIN = 0x10004000`) and deploy via True A/B dual-slot OTA (switching from active Slot B).
- Halt STM32H503 under reset via `POST /api/v1/programmer/reset-catch` and read PC.
- Program BareBoot I3C bootloader (`bareboot-stm32h503-i3c.bin`, 3,216 bytes) into Sector 0 (`0x08000000`).
- Verify bootloader execution via PA0 Blue LED (5 Hz blink) and host I2C/I3C bus scan at static address `0x40`.
- Relocate N20-Servo firmware application to `0x08002000` (120 KB), compile, and flash over BareBoot.

**Non-Goals:**
- Modifying LittleFS user storage partitions (`0x00204000+`).
- Hardware modifications or soldering external BOOT0 jumpers.
- Altering non-ARM debug transports (WCH 1-Wire, QingKe RV-SWD).

## Decisions

### 1. CSW Configuration for Cortex-M33 Secure Debug
* **Decision**: Update `crates/agentla-programmer/src/swd.rs` to set `ADIV5_AP_CSW_DEFAULT = 0x23000012` (`HNONSEC = 0`), add a configurable `csw: u32` field to `CortexM`, and ensure `mem_read32` / `mem_write32` use `self.csw`.
* **Rationale**: Cortex-M33 targets boot in Secure state. Clearing Bit 30 ensures memory transfers generate Secure Privileged AHB transactions, allowing full access to PPB debug registers (`DHCSR`, `DEMCR`, `AIRCR`) and flash control registers. For legacy Cortex-M0/M3/M4/M7 targets, Bit 30 is reserved/ignored, preserving complete backward compatibility.

### 2. Tight-Burst Vector Catch Across Reset Deassertion
* **Decision**: In `CortexM::connect_under_reset()`, immediately after `self.swd.assert_reset(false)`, execute a rapid burst loop (10 iterations, ~1 µs delay) writing `DHCSR` (`0xA05F0003`) and `DEMCR` (`0x01000001` with `VC_CORERESET`).
* **Rationale**: The STM32H503 reset sequencer (`t_RSTTEMPO`) takes 200 to 400 µs. A local burst loop on RP2350 latches vector catch within 20 to 50 µs, halting the core at the reset vector before vector 1 is executed and before flash firmware can disable SWD or enter sleep.

```
       assert_reset(true)             assert_reset(false)
nRST  ─────┐                                ┌────────────────────────
           └────────────────────────────────┘
             20ms reset pulse                 Tight Burst Write Window
                                              (<50µs, within t_RSTTEMPO)
                                              ├── DHCSR: DEBUGEN | HALT
                                              └── DEMCR: VC_CORERESET
                                              ==> Core Halts at Reset Vector
```

### 3. AP 1 Selection on CoreSight DP v2 Detection
* **Decision**: In `detect_target_port()`, if `dpidr == 0x6BA02477` or `0x0BE12477`, switch to AP 1 (`AHB5-AP`) and set `cortex.csw = 0x23000012` before reading DBGMCU IDCODE at `0x44022000`.
* **Rationale**: On STM32H5, AP 0 is APB-AP (`IDR = 0x54770002`) and AP 1 is AHB5-AP (`IDR = 0x84770001`). Reading system memory requires AP 1.

### 4. True A/B Dual-Slot OTA Deployment to Slot A
* **Decision**: Since AgentLA is currently running on Slot B (`ORIGIN = 0x10104000`), compile the updated firmware for Slot A (`ORIGIN = 0x10004000`, without `--features slot-b`), package using `tools/package_ota.py`, and upload via `POST /api/v1/system/firmware/ota`.
* **Rationale**: Follows the strict dual-slot OTA contract. If the updated Slot A firmware ever fails, the bootloader safely falls back to Slot B. LittleFS user storage remains completely untouched.

### 5. Sector 0 Bootloader Flashing & Application Relocation
* **Decision**: Program `bareboot-stm32h503-i3c.bin` (3,216 bytes) into Sector 0 (`0x08000000` to `0x08001FFF`, 8 KB). Relocate N20 Servo application `memory.x` to `0x08002000` with length `120K` (122,880 bytes).
* **Rationale**: Clean separation between the 8 KB BareBoot I3C/I2C bootloader and the 120 KB main motor controller application.

## Risks / Trade-offs

- **[Risk] Target core in deep sleep or pin float causes ACK WAIT during burst**  
  → *Mitigation*: The burst loop performs up to 10 attempts across the reset edge with 1 µs backoff, followed by a 500 ms polling loop that re-arms DHCSR/DEMCR periodically.
- **[Risk] Option Byte write protection on STM32H5 Sector 0**  
  → *Mitigation*: `FlashAlgorithmRunner::stm32h5_unlock()` performs FLASH_KEYR unlocking (`0x45670123`, `0xCDEF89AB`) and clears error flags in FLASH_CCR (`0x40022030`) before erasing Sector 0.
- **[Risk] Dual-slot OTA partition overflow**  
  → *Mitigation*: Slot A partition allocation is 1,024 KB. The release binary size is ~300 KB, well within partition limits.
