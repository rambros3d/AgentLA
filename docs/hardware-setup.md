# AgentLA Hardware Setup Guide

## Overview

AgentLA is an RP2350-based logic analyzer designed for both human debugging via Sigrok/PulseView and AI agent integration through structured text output. This guide covers hardware setup, wiring, level shifting, and probe connections.

## Hardware Requirements

### Core Components

1. **RP2350 Board (Pico 2)** - Main microcontroller board
2. **Custom AgentLA PCB** (optional) - With level shifters for 3.3V/5V/ext Vref
3. **Probe cables** - Hook clips or spring probes for signal connection
4. **USB cable** - USB-C for power and data communication
5. **Target device** - The electronics you want to analyze

### Optional Components

- **ESP32 board** - For signal generation and testing
- **External power supply** - If target requires separate power
- **Level shifter board** - For voltage translation (3.3V ↔ 5V)
- **Breadboard & jumpers** - For prototyping connections

## Pin Mapping

### RP2350 GPIO to Channel Mapping

AgentLA supports up to 24 channels (MAX_CHANNELS=24). The default channel mapping uses GPIO pins 2-9 for the first 8 channels:

| Channel | GPIO Pin | Physical Pin (Pico 2) |
|---------|----------|----------------------|
| 0       | GPIO2    | Pin 4                |
| 1       | GPIO3    | Pin 5                |
| 2       | GPIO4    | Pin 6                |
| 3       | GPIO5    | Pin 7                |
| 4       | GPIO6    | Pin 9                |
| 5       | GPIO7    | Pin 10               |
| 6       | GPIO8    | Pin 11               |
| 7       | GPIO9    | Pin 12               |

**Note:** GPIO0 and GPIO1 are reserved for complex trigger I/O on the custom PCB.

### LED Indicator

- **LED**: GPIO25 (shared with USB D-)
- The Pico 2 has a series resistor that allows LED operation without disrupting USB

## Wiring Connections

### Basic Connection (3.3V Logic)

For devices operating at 3.3V logic levels:

```
AgentLA Channel 0 (GPIO2) → Target Signal 0
AgentLA Channel 1 (GPIO3) → Target Signal 1
AgentLA Channel 2 (GPIO4) → Target Signal 2
AgentLA Channel 3 (GPIO5) → Target Signal 3
...
AgentLA GND → Target GND (common ground required)
```

**Critical:** Always connect GND between AgentLA and the target device!

### Level Shifting (5V Logic)

For devices operating at 5V logic levels, use a level shifter:

```
Target 5V Signal → Level Shifter (HV side) → AgentLA Channel (LV side)
Target GND → Level Shifter GND → AgentLA GND
Level Shifter VCC_HV → Target 5V
Level Shifter VCC_LV → AgentLA 3.3V (from Pico 2)
```

**Recommended level shifters:**
- TXS0108E (8-channel bidirectional)
- BSS138-based MOSFET level shifters
- 74HCT245 (direction-controlled)

### External Voltage Reference

If using the custom AgentLA PCB with external Vref:

1. Set the voltage selector jumper to "EXT"
2. Apply reference voltage to the VREF pin (max 5.5V)
3. All channels will use this reference for logic threshold

## Probe Connections

### Hook Clips

Best for:
- Through-hole component pins
- Header pins
- Wire wraps

Connection:
1. Attach GND clip to target ground
2. Connect signal clips to target signals
3. Ensure good contact and minimal lead length

### Spring Probes

Best for:
- SMD pads
- Fine-pitch components
- Tight spaces

Connection:
1. Place G probe on ground plane/pad
2. Press signal probes onto target pads
3. Use probe holder for stable connection

### Direct Soldering

For permanent/semi-permanent connections:
1. Solder fine wires to target test points
2. Route wires to AgentLA channels
3. Use strain relief to prevent breakage

## Protocol-Specific Wiring

### I2C Analysis

For I2C bus monitoring:

```
SDA (Data) → AgentLA Channel 0 (configurable)
SCL (Clock) → AgentLA Channel 1 (configurable)
GND → Common ground
```

**Important:** I2C uses open-drain with pull-ups. AgentLA can monitor but not drive the bus.

### SPI Analysis

For SPI bus monitoring:

```
SCLK (Clock) → AgentLA Channel (configurable)
MOSI (Master Out Slave In) → AgentLA Channel
MISO (Master In Slave Out) → AgentLA Channel
CS (Chip Select) → AgentLA Channel
GND → Common ground
```

### UART Analysis

For UART/serial monitoring:

```
RX (Receive) → AgentLA Channel (configurable)
TX (Transmit) → AgentLA Channel (optional, for bidirectional)
GND → Common ground
```

### PWM Measurement

For PWM signal analysis:

```
PWM Signal → AgentLA Channel
GND → Common ground
```

## Power Considerations

### Power from USB

AgentLA is powered via USB from the Pico 2 board:
- **Voltage:** 5V from USB
- **Current:** ~100mA typical, 200mA max
- **Pico 2 internal regulator:** 3.3V for RP2350 and logic

### Powering Target Device

- Small sensors/modules: Can be powered from AgentLA 3.3V output (Pin 36)
- Larger devices: Use separate power supply
- **Never back-power** the target through signal pins

### Current Limits

- **GPIO output current:** 12mA per pin (RP2350 spec)
- **Total GPIO current:** 100mA (all pins combined)
- **USB current:** 500mA (USB 2.0 spec)

## Errata E9 Workarounds

The RP2350 has Errata E9 affecting GPIO latchup. AgentLA firmware includes workarounds:

1. **Avoid rapid voltage transitions** on GPIO pins
2. **Use series resistors** (100-220Ω) on signals with fast edges
3. **Check silicon revision** - A4 or later recommended
4. **Limit input voltage** to 3.3V ±10% without level shifting

### Silicon Revision Check

To check your RP2350 silicon revision:
1. Connect AgentLA via USB
2. Open serial terminal (115200 baud)
3. Type: `IDENTIFY`
4. Response includes firmware version and capabilities

## Testing Your Setup

### Basic Connectivity Test

1. Connect AgentLA to computer via USB
2. Open serial terminal (115200 baud, 8N1)
3. Type: `IDENTIFY`
4. Expected response: `AGENTLA v1.0 RP2350 24CH 100MHZ`

### Signal Test with ESP32

1. Connect ESP32 GPIO19 → AgentLA Channel 0 (GPIO2)
2. Connect ESP32 GND → AgentLA GND
3. Flash ESP32 with test firmware (see `test/esp32_generator/`)
4. Run: `CONFIGURE channels=0 rate=1000000`
5. Run: `CAPTURE samples=4096 mode=RAW`
6. Verify data stream in serial terminal

### Self-Test Mode

AgentLA includes a built-in self-test:
1. Connect AgentLA Channel 0 to Channel 4 (GPIO2 to GPIO5) with jumper wire
2. Run: `SELFTEST tx=4 baud=115200 samples=8192 rate=100000`
3. Device will transmit 0x55 pattern and capture it back
4. Verify successful decode in output

## Troubleshooting

### No USB Connection

1. Check USB cable (data + power)
2. Try different USB port
3. Check Device Manager (Windows) or `lsusb` (Linux)
4. Reinstall drivers if needed (TinyUSB CDC)

### No Signal Capture

1. Verify GND connection between AgentLA and target
2. Check channel configuration matches wiring
3. Verify target device is powered and running
4. Try lower sample rate (1MHz) for initial testing

### Erratic Data

1. Check for noise on signal lines
2. Add bypass capacitors (100nF) near target ICs
3. Use shorter probe leads
4. Verify voltage levels are within RP2350 specs (0-3.3V)

### GPIO Latchup (Errata E9)

If device crashes or behaves erratically:
1. Power cycle the device
2. Check silicon revision (A4+ recommended)
3. Add series resistors on fast-edge signals
4. Reduce input slew rate if possible

## Safety Precautions

1. **Never exceed 3.3V** on GPIO pins without level shifting
2. **Always connect GND** before signal wires
3. **Power off target** before making connections
4. **Avoid short circuits** between pins
5. **Use ESD protection** when handling sensitive components

## References

- [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)
- [Pico 2 Pinout](https://datasheets.raspberrypi.com/pico/Pico-Raspberry-Pi-Pinout.pdf)
- [AgentLA Firmware Source](../firmware/)
- [gusmanb/logicanalyzer](https://github.com/gusmanb/logicanalyzer) - Original fork source
