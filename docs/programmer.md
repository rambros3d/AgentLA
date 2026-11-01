# AgentLA Programmer & SWD Debug Guide

## Overview

AgentLA supports two modes for programming and debugging target MCUs:

1. **UART Bootloader Mode** - For targets with UART bootloaders (STM32, RP2040)
2. **SWD Debug Mode** - For SWD-capable targets (STM32, RP2040/RP2350)

**All modes are always available and auto-activate when needed.** No manual mode switching required.

### SWD Transport Drivers (PIO vs Bitbang)

AgentLA provides two SWD transport backends:
- **Bitbang Driver (`PROG DRIVER BITBANG`, default reference)**: SIO bit-twiddling with deterministic timing (~100 kbit/s).
- **PIO Driver (`PROG DRIVER PIO`)**: Vendored Raspberry Pi `debugprobe` PIO engine (`firmware/src/programmer/debugprobe/`):
  - `swd_probe.pio`: Single PIO SM program (`probe`, MIT license) handling SWCLK side-setting and SWDIO tri-state timing.
  - `probe_pio_transport.c`/`.h`: Bit primitives and bounded 10 ms FIFO transport (MIT license).
  - `swd_transfer_pio.c`/`.h`: Ported `SWD_Transfer` transaction engine (Apache-2.0 license) with continuous-clock turnaround, WAIT/FAULT back-off, and protocol re-sync.
  - All vendored components are pinned to Raspberry Pi `debugprobe` commit `3fff5b2`.
- Switch drivers at runtime via `PROG DRIVER BITBANG` or `PROG DRIVER PIO`.
- Power-up request value across both drivers is set to `0x30000000` (`CDBGPWRUPREQ | CSYSPWRUPREQ`).

## Flashing AgentLA Firmware (Updating the Device Itself)

### Preferred Method: picotool in BOOTSEL Mode

1. **Enter BOOTSEL mode** by either:
   - Pressing the BOOTSEL button on the RP2350 while USB is connected
   - **OR** performing a double-reset (press/reset/press/reset quickly within 200ms) — works after `pico_bootsel_via_double_reset` is linked

2. **Flash** from the host machine (not the container's stale `/media/sun/RP2350/`):
   ```bash
   # Find the real mount point on the HOST (not in container!)
   lsblk | grep sdb
   # Device = /dev/sdb1, mount = /run/media/sun/RP2350
   
   # From inside container, write through the HOST mount:
   sudo cp build/AgentLA.uf2 /run/media/sun/RP2350/
   sudo sync
   sleep 3
   ```

3. **Verify** the device rebooted:
   ```bash
   lsusb | grep 1209  # Should show ID 1209:414d
   ```

### Troubleshooting Flashing

- **`/media/sun/RP2350/` is stale in container**: The real mount is at `/run/media/sun/RP2350` (a host symlink through container). Copies to `/media/sun/RP2350/` write to local disk, NOT to the device.
- **picotool can't connect in app mode**: Use mass storage copy instead. `picotool reboot -u` is not yet implemented (see below).
- **Device shows 1209:414d but doesn't respond to HID**: The firmware may have a crash during USB init. Check LED behavior — steady ON means firmware hung during init. Double-reset or manual BOOTSEL will let you reflash.

### Crash-Loop Protection (Boot Counter)

AgentLA stores a boot counter in the last sector of flash. On each boot, the counter increments. If the firmware crashes before the user confirms it works (counter > 3), the device automatically enters BOOTSEL mode so you can reflash.

**After flashing new firmware, always run:**
```
CONFIRM
```
This resets the boot counter to 0. If you forget, the device will enter BOOTSEL after 3 reboots.

**Check boot counter:**
```
VERSION
```
Returns `BOOTS:2/3` (current count / max threshold).

### Firmware Version Commands

| Command | Description |
|---------|-------------|
| `VERSION` | Returns build timestamp, git hash, firmware version, and boot counter |
| `CONFIRM` | Resets boot counter (call after verifying new firmware works) |
| `BOOTSEL` | Enter BOOTSEL mode for USB flashing |
| `REBOOT` | Reboot the device |

## SWD Debug Mode

### Quick Start

```bash
# Check connection (SWD auto-initializes on first command)
SWD TEST

# Read IDCODE
SWD IDCODE

# Program flash
FLASH ERASE 0x08000000 0x4000
FLASH WRITE 0x08000000 AABBCCDD11223344
```

### SWD Commands

| Command | Description | Example |
|---------|-------------|---------|
| `SWD IDCODE` | Read target IDCODE register | `SWD IDCODE` |
| `SWD RESET` | SWD line reset sequence | `SWD RESET` |
| `SWD TEST` | Connectivity check + target detection | `SWD TEST` |
| `SWD PINS` | Show SWD pin mapping | `SWD PINS` |
| `SWD READ DP <addr>` | Read Debug Port register | `SWD READ DP 0x0` |
| `SWD READ AP <addr>` | Read Access Port register | `SWD READ AP 0x00` |
| `SWD WRITE DP <addr> <val>` | Write Debug Port register | `SWD WRITE DP 0x0 0x1E` |
| `SWD WRITE AP <addr> <val>` | Write Access Port register | `SWD WRITE AP 0x00 0x23000012` |

### Flash Commands

| Command | Description | Example |
|---------|-------------|---------|
| `FLASH ERASE <addr> <size>` | Erase flash range (1 KiB page erase for STM32F1; full chip mass erase if range covers entire flash) | `FLASH ERASE 0x08000000 0x4000` |
| `FLASH WRITE <addr> <hex>` | Write hex string to flash (16-bit halfword accesses for STM32F1 with EOP/PGERR verification) | `FLASH WRITE 0x08000000 AABBCCDD` |
| `FLASH READ <addr> <len>` | Read length bytes from flash | `FLASH READ 0x08000000 64` |
| `FLASH VERIFY <addr> <hex>` | Verify flash contents match hex string | `FLASH VERIFY 0x08000000 AABBCCDD` |

### Expected Output for STM32F103 (Blue Pill)

When connecting an STM32F103C8T6 (Blue Pill), `SWD TEST` auto-detects DBGMCU ID `0x410` (Medium Density):
```
OK SWD DPIDR=0x1BA01477 target=STM32F103 (Medium Density) flash=64K ram=20K
```
Erase operations perform 1 KiB page erases via `PER=0x02` instead of full-chip mass erases. Writes use 16-bit CSW halfword accesses (`0x23000011`) and verify `FLASH_SR` for `PGERR` (0x04) / `WRPRTERR` (0x10) flags, aborting with explicit error messages if programming fails.

### Pin Mapping

| AgentLA Pin | GPIO | PIO | Function    | Direction | Target Pin |
|-------------|------|-----|-------------|-----------|------------|
| P_SWCLK     | 4    | 1   | SWCLK       | OUT       | SWCLK      |
| P_SWDIO     | 5    | 1   | SWDIO       | BI        | SWDIO      |
| P_NRST      | 6    | SIO | nRST        | OUT       | nRST       |

### Connections

```
AgentLA          Target
--------         ------
P_SWCLK (4)  →   SWCLK
P_SWDIO (5)  ↔   SWDIO
P_NRST (6)   →   nRST (optional)
GND           →   GND
```

**Note:** Target must be powered externally. AgentLA does not provide power via SWD.

### Supported Targets

#### STM32F1xx (Blue Pill, etc.)

```bash
SWD TEST
# Expected: OK SWD TEST PASS idcode=0x1BA01477 target=STM32F1xx

# Read device ID
SWD READ DP 0x0
# Expected: OK SWD DP[0x0] = 0x1BA01477

# Erase and program
FLASH ERASE 0x08000000 0x4000
FLASH WRITE 0x08000000 <hex_data>
```

#### STM32H5xx (STM32H503, STM32H56x/H57x)

```bash
SWD TEST
# Expected: OK SWD TEST PASS idcode=0x0BE12477 target=STM32H503

# Erase (8 KB sector erase or full chip mass erase)
FLASH ERASE 0x08000000 0x4000

# Write (128-bit quad-word programming with automatic alignment/padding)
FLASH WRITE 0x08000000 <hex_data>

# Verify
FLASH VERIFY 0x08000000 <hex_data>
```

#### RP2040/RP2350

```bash
SWD TEST
# Expected: OK SWD TEST PASS idcode=0x0BC12477 target=RP2040

# Flash read only (erase/write not yet implemented)
FLASH READ 0x10000000 64
```

### Mode Conflict Detection

SWD and UART bootloader share GPIO 4-5. If you try to use SWD while UART is active, you'll get an error:

```
SWD TEST
ERR cannot use SWD while UART is active on GPIO4-5
```

Wait for UART operations to complete, then SWD will auto-activate.

## UART Bootloader Mode

For targets with UART bootloaders (STM32, CH32, NRF):

```bash
# Control power and reset
PROG POWER ON
PROG NRESET PULSE
PROG UART TEST

# STM32 bootloader
PROG STM32 SYNC
PROG STM32 WRITE 0x08000000 <hex_data>
```

See [programmer-interface skill](../openspec/changes/programmer-interface/) for details.

## Troubleshooting & SWD Error Codes

### SWD Error Codes Reference

| Error Code / Message | Raw ACK / Bit Value | Cause & Diagnosis | Recommended Resolution |
|----------------------|---------------------|-------------------|------------------------|
| `ERR SWD TEST FAIL - no target` | `raw_ack=0xE0000000` (ACK=7) | Target did not drive `SWDIO` low during ACK phase. SWDIO remained floating/pulled high. | 1. Check target power (3.3V/5V).<br>2. Check `SWCLK` (GPIO4) and `SWDIO` (GPIO5) wiring.<br>3. Check BOOT0 jumper state.<br>4. Pulse target `NRST`. |
| `ERR SWD WAIT timeout` | `raw_ack=0x40000000` (ACK=2) | Target DP/AP returned `WAIT`. Flash memory controller or CPU bus is busy. | Retry operation, lowering SWD clock frequency (`SWD CLOCK 100`) or waiting for flash erase to complete. |
| `ERR SWD FAULT` | `raw_ack=0x80000000` (ACK=4) | Target DP/AP returned `FAULT`. Invalid memory address or access permissions error. | Verify target memory map address range and unlock option bytes before reading/writing protected flash. |
| `ERR SWD init failed: <code >` | N/A | Failed to claim PIO1 state machine or program offsets. | Verify PIO resource allocation. Reset AgentLA firmware (`REBOOT`). |
| `ERR cannot use SWD while UART is active on GPIO4-5` | N/A | Mode conflict: UART programmer mode is active on shared pins GPIO4/5. | Wait for UART session to close or issue `PROG MODE IDLE`. |

### Step-by-Step Troubleshooting

#### 1. Target Connection (ACK = 7)
1. **Verify Wiring**: Confirm `SWCLK` (GPIO4), `SWDIO` (GPIO5), and `GND` connections using a multimeter or `LOOPBACK` command.
2. **Check Target Power**: AgentLA does not supply VCC over SWD pins. Target must be powered externally via 3.3V or USB.
3. **Reset Target**: Execute `SWD RESET` to issue 50+ SWCLK cycles, JTAG-to-SWD select sequence (`0x79E7`), and pulse `NRST`.
4. **Check BOOT0 State**: On STM32 targets, ensure `BOOT0` is set appropriately and the debug port is not disabled by SWD pin remapping in target firmware.

#### 2. Flash ERASE/WRITE Failure
1. Verify memory alignment (flash programming requires 16-bit or 32-bit aligned addresses).
2. Check flash unlock key sequence (`FLASH_KEY1 = 0x45670123`, `FLASH_KEY2 = 0xCDEF89AB`).
3. Ensure flash sectors are erased prior to writing (`FLASH ERASE`).

#### 3. Shared Pin Mode Conflicts
- SWD and UART bootloader share GPIO 4 and 5.
- The firmware automatically prevents simultaneous usage and returns `ERR cannot use SWD while UART is active on GPIO4-5`.

### STM32F1 Automated 5-Tier Hardware Verification Suite

AgentLA includes an end-to-end automated 5-tier test harness to verify physical probe capabilities on STM32F1 (Blue Pill) targets:

```bash
python3 test/test_stm32f1_hardware_suite.py /dev/ttyACM0
```

- **Tier 1: Transport & Protocol** — Validates line reset, `0xE79E` JTAG-to-SWD switch, DPIDR `0x1BA01477`, `DP_CTRLSTAT` power-up, AP IDR `0x04770001`, and SWD clock rates (100 kHz - 5 MHz).
- **Tier 2: Cortex-M3 Core Control** — Validates DHCSR CPU Halt (`0xA05F0003`), Unhalt (`0xA05F0000`), core register reading, and vector catch.
- **Tier 3: AHB System Bus & Chip ID** — Validates SRAM 8/16/32-bit read/write, DBGMCU ID `0x410` (Medium Density), Flash size geometry (`0x1FFFF7E0`), and 96-bit Unique Chip ID (`0x1FFFF7E8`).
- **Tier 4: FPEC Flash Controller** — Validates FPEC key unlock (`0x45670123`/`0xCDEF89AB`), 1 KiB page erase, 16-bit CSW halfword programming, and `PGERR`/`WRPRTERR` status flag clearing.
- **Tier 5: Signal Capture & Application** — Flashes PC13 blink firmware, verifies byte readback, issues system reset via `AIRCR`, and uses the AgentLA logic analyzer engine on `GPIO18` to verify the 1 Hz LED toggle output.

#### Diagnostic CLI Command: `STM32 TEST`
You can also issue `STM32 TEST` directly over serial CDC terminal:
```
> STM32 TEST
OK STM32 TEST PASS idcode=0x1BA01477 target=STM32F103 (Medium Density) flash=64K ram=20K
```



## OpenOCD Configuration Examples

### Basic OpenOCD Connection

AgentLA can be used as an SWD adapter with OpenOCD. Here are example configurations:

#### STM32F1xx (Blue Pill)

```bash
# AgentLA SWD interface
openocd -f interface/agentla-swd.cfg \
        -f target/stm32f1x.cfg \
        -c "adapter speed 1000"
```

#### RP2040/RP2350

```bash
openocd -f interface/agentla-swd.cfg \
        -f target/rp2040.cfg \
        -c "adapter speed 1000"
```

### OpenOCD Interface Configuration

Create `agentla-swd.cfg`:

```
# AgentLA SWD Interface
interface remote_bitbang
remote_bitbang_host localhost
remote_bitbang_port 3333

# SWD Pins
adapter speed 1000

# Reset configuration
reset_config srst_only
```

### Using AgentLA CLI with OpenOCD

```bash
# SWD auto-activates when you run commands
SWD TEST

# Start OpenOCD (connects via AgentLA CLI)
openocd -f interface/agentla-swd.cfg -f target/stm32f1x.cfg

# In OpenOCD, you can then:
# - Read target info: targets
# - Flash firmware: program firmware.elf verify reset exit
# - Debug: gdb_port 3333
```

## pyOCD Configuration Examples

### Python Script for Flash Programming

```python
import serial
import time

def agentla_swd_program(port, firmware_path):
    """Program firmware using AgentLA SWD interface."""
    ser = serial.Serial(port, 115200, timeout=2)
    
    # Test connection (SWD auto-initializes)
    ser.write(b"SWD TEST\r\n")
    response = ser.readline().decode().strip()
    print(f"Test: {response}")
    
    if "PASS" not in response:
        print("Target not connected!")
        return False
    
    # Read firmware
    with open(firmware_path, 'rb') as f:
        firmware = f.read()
    
    # Erase flash
    ser.write(f"FLASH ERASE 0x08000000 {len(firmware):#x}\r\n".encode())
    response = ser.readline().decode().strip()
    print(f"Erase: {response}")
    
    # Program flash (hex encoded)
    hex_data = firmware.hex()
    ser.write(f"FLASH WRITE 0x08000000 {hex_data}\r\n".encode())
    response = ser.readline().decode().strip()
    print(f"Write: {response}")
    
    # Verify
    ser.write(f"FLASH VERIFY 0x08000000 {hex_data}\r\n".encode())
    response = ser.readline().decode().strip()
    print(f"Verify: {response}")
    
    ser.close()
    
    return "PASS" in response

# Usage
if __name__ == "__main__":
    agentla_swd_program("/dev/ttyACM0", "firmware.bin")
```

### pyOCD Target Detection

```python
import serial
import time

def detect_target(port):
    """Detect connected target via AgentLA SWD."""
    ser = serial.Serial(port, 115200, timeout=2)
    
    ser.write(b"SWD IDCODE\r\n")
    response = ser.readline().decode().strip()
    print(f"IDCODE: {response}")
    
    ser.write(b"SWD TEST\r\n")
    response = ser.readline().decode().strip()
    print(f"Target: {response}")
    
    ser.close()
    
    return response

# Usage
if __name__ == "__main__":
    detect_target("/dev/ttyACM0")
```

### Flash Read/Write Example

```python
import serial

def flash_operations(port):
    """Demonstrate flash operations via AgentLA SWD."""
    ser = serial.Serial(port, 115200, timeout=2)
    
    # Read flash
    ser.write(b"FLASH READ 0x08000000 64\r\n")
    data = ser.readline().decode().strip()
    print(f"Flash data: {data}")
    
    # Erase sector
    ser.write(b"FLASH ERASE 0x08000000 0x400\r\n")
    response = ser.readline().decode().strip()
    print(f"Erase: {response}")
    
    # Write data
    test_data = "AABBCCDD" * 8  # 32 bytes
    ser.write(f"FLASH WRITE 0x08000000 {test_data}\r\n".encode())
    response = ser.readline().decode().strip()
    print(f"Write: {response}")
    
    # Verify
    ser.write(f"FLASH VERIFY 0x08000000 {test_data}\r\n".encode())
    response = ser.readline().decode().strip()
    print(f"Verify: {response}")
    
    ser.close()
```

## Troubleshooting OpenOCD/pyOCD

### Common Issues

1. **Connection fails**
   - Check SWD wiring (SWCLK, SWDIO, GND)
   - Ensure target is powered
   - Try lower adapter speed: `-c "adapter speed 100"`

2. **IDCODE read fails**
   - Target may not support SWD
   - Try `SWD RESET` before `SWD IDCODE`
   - Check BOOT0 pin state (must be LOW for SWD)

3. **Flash operations fail**
   - Some targets require specific unlock sequences
   - Try erasing entire flash before programming
   - Check flash algorithm support in firmware

4. **OpenOCD timeout**
   - Increase timeout: `-c "adapter speed 100"`
   - Check for electrical noise on SWD lines
   - Use shorter cables

### Debug Tips

```bash
# Enable verbose OpenOCD output
openocd -d3 -f interface/agentla-swd.cfg -f target/stm32f1x.cfg

# Read all registers
SWD READ DP 0x0
SWD READ AP 0x00
```
