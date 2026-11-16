## Context

AgentLA runs on a dual-core Raspberry Pi Pico 2 (RP2350) and integrates a PIO-based CMSIS-DAP v1 (HID) and v2 (USB Bulk) debug probe. It is connected to an STM32H503KBU6 target (ARM Cortex-M33 with TrustZone) over SWD:
- GP4: SWCLK
- GP5: SWDIO
- GP6: nRST

During previous testing, DP read operations (`DPIDR`) succeeded (`0x6BA02477`), but write transactions (`DP_SELECT`, `DP_CTRLSTAT`) produced `ack=4` (FAULT ACK) and set `WDATAERR` (`0x08000080`), blocking host-side tools (`pyocd`, `probe-rs`, GDB server) from attaching. Furthermore, direct flashing over HTTP failed because STM32H5 quad-word programming was not implemented in the target engine, and Slot B firmware retained an active diagnostic intercept in `read_flash_port_internal`.

Cross-referencing the `/ref/hybridlink` reference implementation (`ref/hybridlink/app/swd.c` and `dap.c`) confirmed:
1. `crates/agentla-programmer/src/pio/swd.rs` configured the PIO state machine `pinctrl` with `sideset_count = 2`, whereas `SWD_INSTRUCTIONS` was assembled from `probe.pio` which specifies `.side_set 1 opt`. Because of this mismatch, sideset was driving both GP4 (SWCLK) and GP5 (SWDIO) on every sideset cycle, corrupting outgoing write data and parity.
2. `hybridlink` mandates unconditionally clocking at least 2 trailing idle zero bits (`write_bits(0, 2)`) after every transaction. AgentLA's `transfer_raw` only outputted idle clocks when `idle_cycles > 0`, which defaults to 0 in CMSIS-DAP, leaving CoreSight write buffers un-flushed.
3. `dap.rs` was missing `MATCH_VALUE` and `MATCH_MASK` handling, which causes desynchronization when host tools poll registers during halting and flash polling.
4. Memory transfers must break at 1 KiB boundaries (`1024 - (addr & 1023)`) to prevent CoreSight 10-bit `TAR` address wrapping.

## Goals / Non-Goals

**Goals:**
- Fix PIO state machine sideset count to 1 (`SWCLK` only) and enforce at least 2 trailing idle cycles per transfer, eliminating write data corruption and `WDATAERR`.
- Implement `MATCH_VALUE` and `MATCH_MASK` support in `dap.rs` for reliable host tool status register polling.
- Verify reliable CMSIS-DAP packet handling (all 12 checks in `test/test_cmsis_dap.py` passing with `ack=1`).
- Verify GDB server connectivity with both `pyocd gdbserver` and `probe-rs gdb`.
- Implement autonomous STM32H5 flash driver in `crates/agentla-targets/src/engines/stm32.rs` supporting unlock, sector erase, 128-bit quad-word writes, and 1 KiB TAR chunking.
- Clean up diagnostic stubs in `firmware-rs/src/programmer/mod.rs` and verify direct memory read/flash via HTTP REST APIs.
- Build Slot A firmware (`0x10004000`), package OTA, deploy to `10.10.10.10`, and verify dual-slot fail-safe health.

**Non-Goals:**
- Changing hardware wiring or board pinout.
- Implementing STM32H5 option byte editing or TrustZone security reconfiguration (flashing will operate in standard non-secure/secure application space).
- Replacing Embassy async runtime or LittleFS partitions.

## Decisions

### 1. PIO Sideset Pin Count: 1 Pin (SWCLK Only)
- **Decision**: Configure `w.set_sideset_count(1)` with `sideset_base = clk_pin` in `init_pins_split`.
- **Rationale**: `SWD_INSTRUCTIONS` comes from `probe.pio` where sideset controls only the clock pin (`.side_set 1 opt`). When `sideset_count` was set to 2, bits 11:10 of instructions were treated as 2 sideset pins, erroneously driving GP5 (SWDIO) and truncating instruction delay. Setting count to 1 aligns hardware with the instruction encoding.
- **Alternatives Considered**: Modifying instruction words to support 2 sideset pins. Rejected because standard SWD on AgentLA uses bidirectional SWDIO with dynamic direction switching, which does not require a dedicated external output-enable pin like `probe_oen.pio`.

### 2. Mandatory Trailing Idle Cycles (Minimum 2 Cycles)
- **Decision**: In `transfer_raw`, clock out `max(self.idle_cycles, 2)` idle zero cycles at the end of each transfer.
- **Rationale**: Follows `hybridlink`'s verified behavior (`write_bits(0, 2)`). The ARM ADIv5 specification specifies that trailing idle cycles allow CoreSight DP write buffers to drain into internal bus interconnects before the line is turned around or the next command header is issued.
- **Alternatives Considered**: Relying solely on host-configured `idle_cycles`. Rejected because host tools default `idle_cycles` to 0, which triggers race conditions on high-speed SWD.

### 3. CMSIS-DAP Value Match Support in `dap.rs`
- **Decision**: Support `MATCH_VALUE` (`0x10`) and `MATCH_MASK` (`0x20`) in `handle_transfer`.
- **Rationale**: Host debuggers like OpenOCD and pyOCD use `DAP_Transfer` match commands to wait for `FLASH_SR` BSY bits or `DHCSR` halt state. Without match handling, the 4-byte match payload is misinterpreted as the next transfer request, corrupting the packet parse offset.
- **Alternatives Considered**: Rejecting match commands with DAP_TRANSFER_ERROR. Rejected because pyOCD halts execution if value matching is rejected.

### 4. Flash Controller Implementation: AHB-AP Direct Register Driver with 1 KiB TAR Chunking
- **Decision**: Implement STM32H5 flash programming directly via CoreSight AHB-AP (AP 1) register writes to `FLASH_NSCR` (`0x40022028`), `FLASH_NSKEYR` (`0x40022004`), and quad-word memory writes to `0x08000000`, chunking transfers at 1 KiB boundaries (`1024 - (addr & 1023)`).
- **Rationale**: The STM32H5 internal flash controller requires unlocking with keys `0x45670123` and `0xCDEF89AB`, followed by writing 4 consecutive 32-bit words (128 bits) while `PG` is asserted. Executing this directly via AHB-AP requires no target RAM loader stub and works deterministically while target core is halted. 1 KiB chunking avoids TAR auto-increment wrap.
- **Alternatives Considered**: Uploading a CMSIS Flash Algorithm ELF to target SRAM. Rejected because SRAM requires setup of vector tables and stack pointers under TrustZone, whereas direct AHB-AP writes are self-contained and zero-dependency.

### 5. Slot A OTA Deployment with A/B Rollback
- **Decision**: Build release firmware for Slot A (`ORIGIN = 0x10004000`), package with `package_ota.py`, and upload to `POST /api/v1/firmware/ota`.
- **Rationale**: AgentLA is currently running on Slot B. Writing to Slot A preserves the working bootloader and LittleFS storage (`0x00204000+`) according to the True A/B Dual-Slot OTA contract.

## Risks / Trade-offs

- **[Risk] Pre-existing firmware reclaims SWD pins upon boot**
  - *Mitigation*: Ensure `connect_under_reset` maintains vector catch (`DEMCR_VC_CORERESET` + `DHCSR_C_DEBUGEN | DHCSR_C_HALT`) across reset release, capturing the core before flash user code can disable PA13/PA14.
- **[Risk] High-speed SWD clock causes signal ringing on flying jumper wires**
  - *Mitigation*: Maintain default clock at 1,000 kHz (1 MHz) for connection, with fallback down to 100 kHz during reset recovery.
- **[Risk] Flash programming timeout on quad-word writes**
  - *Mitigation*: Poll `FLASH_NSSR` `BSY` bit with a 100 ms timeout per quad-word and clear any error flags (`WRPERR`, `PGSERR`) via `FLASH_NSCCR`.

## Migration Plan

1. Edit `crates/agentla-programmer/src/pio/swd.rs` to fix `sideset_count` to 1 and ensure at least 2 trailing idle cycles.
2. Edit `crates/agentla-programmer/src/dap.rs` to add `MATCH_VALUE` and `MATCH_MASK` handling.
3. Edit `firmware-rs/src/programmer/mod.rs` to clean up `read_flash_port_internal`.
4. Add STM32H5 flash engine to `crates/agentla-targets/src/engines/stm32.rs`.
5. Compile `firmware-rs` for Slot A:
   ```bash
   cargo build --release -p firmware-rs --target thumbv8m.main-none-eabihf
   python3 tools/package_ota.py target/thumbv8m.main-none-eabihf/release/firmware-rs firmware.ota.bin
   curl -X POST --data-binary @firmware.ota.bin http://10.10.10.10/api/v1/firmware/ota
   ```
6. Run automated verification suite (`test_cmsis_dap.py`, pyocd gdbserver test, HTTP read/flash test).
