# CH32V203 Hardware Test Instructions

## Prerequisites

1. **Hardware Required:**
   - AgentLA board (RP2350 Pico 2)
   - CH32V203 target board
   - USB cable for AgentLA
   - SWD connections between AgentLA and CH32V203

2. **Software Required:**
   - Python 3 with pyserial: `pip install pyserial`
   - Firmware already built: `build/AgentLA.uf2`

## Wiring

Connect AgentLA to CH32V203 via SWD:

| AgentLA Pin | CH32V203 Pin | Description |
|-------------|--------------|-------------|
| GPIO 4      | SWCLK        | SWD Clock   |
| GPIO 5      | SWDIO        | SWD Data    |
| GPIO 6      | nRST         | Reset (optional) |
| GND         | GND          | Ground      |

**Important:** CH32V203 must be powered externally (3.3V).

## Flash Firmware

1. Put AgentLA in BOOTSEL mode:
   - Hold BOOTSEL button on Pico 2
   - Connect USB cable
   - Release button

2. Copy firmware:
   ```bash
   cp build/AgentLA.uf2 /media/$USER/RPI-RP2/
   ```

3. Wait for device to reboot (LED should blink)

## Run Tests

### Quick Test (5 stages)
```bash
cd test
python3 test_ch32v203_hardware_verification.py /dev/ttyACM0
```

### Full Test (10 stages + blink firmware)
```bash
cd test
python3 test_ch32v203_swd_blink.py /dev/ttyACM0
```

## MCU Selection (manual)

Auto-detection was removed — the user always selects the MCU model explicitly. The model picks the debug transport and flash parameters:

| Command | Transport | Target |
|---------|-----------|--------|
| `CH32 DETECT mcu=ch32v003` | SWIO 1-wire | CH32V003 (16 KiB, 64 B pages) |
| `CH32 DETECT mcu=ch32v203` | RVSWD 2-wire | CH32V203 (64 KiB, 256 B pages) |
| `CH32 DETECT mcu=ch32v307` | RVSWD 2-wire | CH32V307 (256 KiB, 256 B pages) |

Legacy aliases remain for test-script compatibility: `mode=1wire` → `mcu=ch32v003` defaults, `mode=2wire` → `mcu=ch32v203` defaults.

## Expected Output

### Quick Test
```
============================================================
 AgentLA CH32V203 2-Wire RV-SWD Hardware Verification Suite
============================================================
Target Serial Port: /dev/ttyACM0

--- Stage 1: AgentLA Firmware Handshake ---
  AGENTLA v1.0 RP2350 ANALYZER+ADAPTER+SWD 10CH 100MHZ I2C+UART+SWD SIMULTANEOUS
  BUILD:2026-08-03 18:30:00 abc1234
  ✓ Stage 1 Passed: AgentLA Firmware Handshake Successful

--- Stage 2: Target Probe & ESIG Match (2-Wire RV-SWD) ---
  OK CH32 DETECT chip_id=0x20305040 flash_size=65536 page_size=256 mode=2wire
  ✓ Stage 2 Passed: CH32V203 ESIG Probe Successful
--- Stage 3: Flash Sector Erase (128 Bytes at 0x08000000) ---
  OK CH32 ERASED 128 bytes at 0x08000000
  ✓ Stage 3 Passed: Flash Page Erase Successful

--- Stage 4: Fast Mode Flash Write (128 Bytes Payload) ---
  OK CH32 FLASH WROTE 128 bytes at 0x08000000
  ✓ Stage 4 Passed: Fast Page Write Successful

--- Stage 5: Flash Readback & Byte Parity Verification ---
  FLASH DATA 78563412f0debc9a3322110077665544...
  ✓ Byte Parity Verification Match: 128 bytes match payload exactly!

============================================================
 SUCCESS: ALL CH32V203 HARDWARE VERIFICATION STAGES PASSED! ✓
============================================================
```

### Full Test (with blink)
Additional stages will flash a blink firmware and verify it runs on CH32V203.

## Troubleshooting

### "ERR CH32 DETECT failed"
- Check SWD wiring (SWCLK, SWDIO, GND)
- Ensure CH32V203 is powered (3.3V)
- Try resetting AgentLA (unplug/replug USB)

### "ERR SWD init failed"
- Check for short circuits on SWD pins
- Ensure no other debugger is connected
- Try lower SWD frequency (not implemented yet)

### Flash verification fails
- Check power supply stability
- Try shorter SWD cable
- Ensure CH32V203 is not in read-protection mode
