# AgentLA LLM Agent Integration Guide

## Overview

AgentLA is designed from the ground up for LLM agent integration. Unlike traditional logic analyzers that stream raw waveform data, AgentLA performs on-device protocol decode and outputs structured text that language models can directly parse and reason about.

This guide explains how to integrate AgentLA with AI agents for automated electronics debugging.

## Communication Protocol

### Transport

AgentLA uses **USB CDC serial** (virtual COM port) for communication:
- **Baud rate:** 115200 (default, configurable)
- **Data bits:** 8
- **Stop bits:** 1
- **Parity:** None
- **Line endings:** `\r\n` (CR+LF)

### Command Format

All commands follow the pattern:
```
COMMAND [param=value ...]\r\n

```

### Response Format

Responses are text lines terminated by `\r\n`:
- **Echo:** `> COMMAND\r\n` (command echo)
- **Response:** `DATA\r\n` (result data)
- **Error:** `ERR description\r\n` (error message)
- **Events:** `PROTOCOL EVENT [data]\r\n` (decoded protocol events)

## Quick Start for Agents

### Step 1: Discover Device

```python
import serial
import time

# Open serial connection
ser = serial.Serial('/dev/ttyACM0', 115200, timeout=2)
time.sleep(1)  # Wait for device reset

# Identify device
ser.write(b'IDENTIFY\r\n')
response = ser.readline().decode().strip()  # Skip echo
response = ser.readline().decode().strip()  # Get actual response
print(response)
# Output: AGENTLA v1.0 RP2350 24CH 100MHZ
```

### Step 2: Configure Channels

```python
# Configure 4 channels at 10MHz sample rate
ser.write(b'CONFIGURE channels=0,1,2,3 rate=10000000\r\n')
# Skip echo
ser.readline()
# Get response
response = ser.readline().decode().strip()
print(response)
# Output: OK configured 4 channels at 10000000 Hz
```

### Step 3: Capture and Decode

```python
# Capture I2C traffic
ser.write(b'CAPTURE samples=4096 mode=DECODED protocol=I2C sda=0 scl=1\r\n')

# Read response lines
while True:
    line = ser.readline().decode().strip()
    if not line:
        continue
    if line.startswith('>'):
        continue  # Skip echo
    if line.startswith('DONE'):
        print(f"Capture complete: {line}")
        break
    if line.startswith('ERR'):
        print(f"Error: {line}")
        break
    # Parse decoded event
    print(f"Event: {line}")
    # Output examples:
    # 0.000123 I2C START
    # 0.000125 I2C ADDR 0x50 W
    # 0.000127 I2C ACK
    # 0.000130 I2C DATA 0xFF
    # 0.000132 I2C STOP
```

## Complete Python SDK Example

### Installation

Install from source (package not yet published to PyPI):

```bash
git clone https://github.com/AgentLA/AgentLA
cd AgentLA/sdk
pip install -e .
```

### Basic Usage

```python
import agentla

# Auto-detect and connect
with agentla.connect() as device:
    # Identify
    info = device.identify()
    print(f"Device: {info.model} {info.channels}ch @ {info.max_sample_rate/1e6:.0f}MHz")
    
    # Configure for I2C analysis
    device.configure(
        channels=[0, 1],
        sample_rate=1_000_000,  # 1MHz for I2C
        trigger=None
    )
    
    # Capture decoded I2C
    events = device.capture_decoded(
        protocol='I2C',
        sda=0,  # Channel 0 = GPIO2
        scl=1,  # Channel 1 = GPIO3
        samples=4096
    )
    
    # Process events
    for event in events:
        if event.type == 'START':
            print(f"Transaction start at {event.timestamp:.6f}s")
        elif event.type == 'ADDR':
            rw = 'R' if event.rw else 'W'
            print(f"Address: 0x{event.value:02X} ({rw})")
        elif event.type == 'DATA':
            print(f"Data: 0x{event.value:02X}")
        elif event.type == 'STOP':
            print(f"Transaction end")
```

### Raw Capture for Sigrok

```python
import agentla

with agentla.connect() as device:
    device.configure(channels=[0,1,2,3], sample_rate=10_000_000)
    
    # Raw capture (hex-encoded pin states)
    capture = device.capture_raw(
        samples=4096,
        trigger={'pin': 0, 'edge': 'rising'}
    )
    
    # Export to Sigrok format
    capture.export_sigrok('capture.sr')
    
    # Or access raw data
    print(f"Got {len(capture.data)} samples")
    print(f"Sample rate: {capture.sample_rate} Hz")
    print(f"Duration: {capture.duration:.6f} seconds")
```

## Agent Prompt Templates

### Template 1: Analyze Unknown Signal

```
You are an electronics debugging assistant. I have connected a logic analyzer 
to a device and captured signal data. Please analyze the following capture and 
identify the communication protocol and any issues.

Device response:
{paste AgentLA output here}

Please:
1. Identify the protocol (I2C, SPI, UART, etc.)
2. List any errors or warnings
3. Decode the data payload
4. Suggest next debugging steps
```

### Template 2: Debug I2C Communication

```
I'm debugging an I2C device that's not responding correctly. Here's the 
logic analyzer capture:

{paste AgentLA I2C decode output}

The device address should be 0x50. Please:
1. Verify the device is ACKing its address
2. Check for clock stretching issues
3. Identify any NACK conditions
4. Suggest why communication might be failing
```

### Template 3: Verify SPI Configuration

```
I'm configuring an SPI flash memory chip. Please verify my SPI settings 
are correct based on this capture:

{paste AgentLA SPI decode output}

Expected: CPOL=0, CPHA=0, 8MHz clock, MSB first
Please:
1. Verify clock polarity and phase
2. Check MOSI/MISO data validity
3. Verify CS assertion timing
4. Confirm byte order matches expectations
```

## Advanced Agent Techniques

### 1. Adaptive Sample Rate Selection

Agents can automatically select optimal sample rates based on detected protocol:

```python
def auto_configure_for_protocol(device, protocol_hint):
    """Auto-configure sample rate based on expected protocol."""
    rate_map = {
        'I2C_100K': 1_000_000,    # 10x oversampling
        'I2C_400K': 4_000_000,    # 10x oversampling
        'SPI_1M': 10_000_000,     # 10x oversampling
        'SPI_10M': 100_000_000,   # 10x oversampling (max)
        'UART_9600': 96_000,      # 10x oversampling
        'UART_115200': 1_152_000, # 10x oversampling
    }
    
    rate = rate_map.get(protocol_hint, 1_000_000)
    device.configure(sample_rate=rate)
    return rate
```

### 2. Error Recovery Strategy

Agents should handle common errors gracefully:

```python
def safe_capture(device, **kwargs):
    """Capture with automatic error recovery."""
    max_retries = 3
    
    for attempt in range(max_retries):
        try:
            result = device.capture_decoded(**kwargs)
            return result
        except DeviceError as e:
            if 'ERR exceeds max capture time' in str(e):
                # Reduce sample count
                kwargs['samples'] = kwargs.get('samples', 4096) // 2
                print(f"Reducing samples to {kwargs['samples']}")
            elif 'ERR invalid GPIO' in str(e):
                # Fix pin mapping
                print("Invalid pin, using default mapping")
                kwargs['sda'] = 0
                kwargs['scl'] = 1
            else:
                raise
    
    raise RuntimeError("Capture failed after retries")
```

### 3. Multi-Protocol Analysis

Analyze multiple protocols simultaneously:

```python
def analyze_mixed_bus(device):
    """Analyze a bus with multiple protocols."""
    
    # First pass: I2C on channels 0-1
    device.configure(channels=[0, 1], sample_rate=1_000_000)
    i2c_events = device.capture_decoded(
        protocol='I2C', sda=0, scl=1, samples=4096
    )
    
    # Second pass: UART on channel 2
    device.configure(channels=[2], sample_rate=1_152_000)
    uart_events = device.capture_decoded(
        protocol='UART', rx=2, baud=115200, samples=4096
    )
    
    # Merge and correlate by timestamp
    all_events = sorted(
        i2c_events + uart_events,
        key=lambda e: e.timestamp
    )
    
    return all_events
```

## Protocol-Specific Examples

### I2C EEPROM Read

```python
# Read 16 bytes from EEPROM at address 0x50
device.configure(channels=[0, 1], sample_rate=1_000_000)

events = device.capture_decoded(
    protocol='I2C',
    sda=0,  # SDA on channel 0
    scl=1,  # SCL on channel 1
    samples=8192
)

# Parse EEPROM read sequence
address = None
data_bytes = []
for event in events:
    if event.type == 'ADDR':
        address = event.value
        rw = 'read' if event.rw else 'write'
    elif event.type == 'DATA' and rw == 'read':
        data_bytes.append(event.value)

print(f"Read {len(data_bytes)} bytes from 0x{address:02X}")
print(f"Data: {[f'0x{b:02X}' for b in data_bytes]}")
```

### SPI Flash ID

```python
# Read JEDEC ID from SPI flash
device.configure(
    channels=[0, 1, 2, 3],
    sample_rate=10_000_000
)

events = device.capture_decoded(
    protocol='SPI',
    cs=0,     # CS on channel 0
    sclk=1,   # SCLK on channel 1
    mosi=2,   # MOSI on channel 2
    miso=3,   # MISO on channel 3
    samples=2048
)

# Extract MOSI command and MISO response
mosi_data = []
miso_data = []
for event in events:
    if event.type == 'MOSI':
        mosi_data.append(event.value)
    elif event.type == 'MISO':
        miso_data.append(event.value)

print(f"Command sent: {[f'0x{b:02X}' for b in mosi_data]}")
print(f"Response: {[f'0x{b:02X}' for b in miso_data]}")
# Typical response: manufacturer ID, memory type, capacity
```

### UART Debug Console

```python
# Monitor debug UART output from target device
device.configure(channels=[0], sample_rate=1_152_000)

events = device.capture_decoded(
    protocol='UART',
    rx=0,           # RX on channel 0
    baud=115200,
    samples=32768   # Capture more for text output
)

# Reconstruct text
text = ''
for event in events:
    if hasattr(event, 'byte') and event.error is None:
        text += chr(event.byte)

print("Target device output:")
print(text)
```

### ADC Voltage Measurement

```python
# Measure analog voltage on ADC pin 26
ser.write(b'ADC pin=26 rate=100000 samples=1024\r\n')

# Skip echo and CAPTURING response
ser.readline()  # Echo
ser.readline()  # ADC CAPTURING...

# Read ADC values
adc_values = []
while True:
    line = ser.readline().decode().strip()
    if not line:
        continue
    if line.startswith('ADC DONE'):
        break
    if line.startswith('ADC '):
        value = int(line.split()[1])
        adc_values.append(value)

print(f"Collected {len(adc_values)} ADC samples")
# ADC values are 12-bit (0-4095) representing 0-3.3V
```

## Error Handling Reference

### Common Errors

| Error | Meaning | Agent Action |
|-------|---------|--------------|
| `ERR invalid GPIO pin` | Pin number out of range | Use valid GPIO (0-29) |
| `ERR too many channels, max 24` | Channel count exceeded | Reduce to ≤24 channels |
| `ERR sample rate exceeds maximum 100000000` | Rate too high | Reduce sample rate |
| `ERR capture configuration error` | Invalid capture params | Check all parameters |
| `ERR DECODED mode requires protocol=...` | Missing protocol in DECODED mode | Add protocol parameter |
| `ERR unknown protocol` | Invalid protocol name | Use I2C, SPI, UART, or PWM |
| `ERR unknown command: ...` | Unrecognized command | Check command spelling |
| `ERR ADC pin must be 26-29` | Invalid ADC pin | Use GPIO 26-29 only |

### Recovery Strategies

1. **Rate too high:** Reduce to 10MHz or lower for initial testing
2. **Pin mapping error:** Use `gpio_to_pindex()` to verify pin mapping
3. **Capture timeout:** Reduce sample count or increase buffer
4. **Decode errors:** Verify pin assignments match actual wiring

## Best Practices for Agent Authors

### 1. Start Simple

Always begin with basic identification and simple captures:
```
IDENTIFY
CONFIGURE channels=0 rate=1000000
CAPTURE samples=1024 mode=RAW
```

### 2. Verify Configuration

After CONFIGURE, verify settings with STATUS:
```
STATUS
```

The device will respond with one of:
- `STATUS IDLE` - Device is ready for commands
- `STATUS CAPTURING` - Capture is in progress
- `STATUS DECODING` - On-device protocol decode is in progress

### 3. Progressive Complexity

Increase sample rate and complexity gradually:
1. Start at 1MHz sample rate
2. Verify basic capture works
3. Increase rate and add protocol decode
4. Add triggering for specific patterns

### 4. Error Context

When reporting errors to users, include:
- Device response
- Configuration used
- Wiring diagram (if available)
- Suggested fixes

### 5. Document Findings

Always log captures for future reference:
```python
# Save capture with metadata
capture.save_with_metadata('debug_session_001.json', {
    'timestamp': datetime.now().isoformat(),
    'target_device': 'STM32F103',
    'protocol': 'I2C',
    'issue': 'Device not ACKing',
    'wiring': 'SDA=GPIO2, SCL=GPIO3'
})
```

## Command Reference

### IDENTIFY
Returns device model, firmware version, channel count, and capabilities.

### CONFIGURE
Configure capture channels, sample rate, and trigger settings.
- `channels=0,1,2,3` - GPIO pins to capture
- `rate=10000000` - Sample rate in Hz
- `trigger=RISING|FALLING|PATTERN` - Trigger type
- `pin=0` - Trigger pin
- `pattern=0xFF` - Pattern trigger value

### CAPTURE
Start data capture with specified mode.
- `samples=4096` - Number of samples
- `mode=RAW|DECODED` - Capture mode
- `pretrigger=50` - Pre-trigger percentage
- `protocol=I2C|SPI|UART|PWM` - Protocol for DECODED mode
- Protocol-specific params: `sda=`, `scl=`, `cs=`, `sclk=`, `mosi=`, `miso=`, `rx=`, `baud=`

### MEASURE
Measure PWM signal properties.
- `MEASURE PWM pin=0` - Measure PWM on specified pin

### ADC
Sample analog voltage on ADC-capable pins.
- `ADC pin=26 rate=100000 samples=1024`
- Pins: 26-29 only (RP2350 ADC inputs)
- Max rate: 500kHz
- Max samples: 32768

### STATUS
Returns current device state: IDLE, CAPTURING, or DECODING.

## Integration with MCP (Model Context Protocol)

For agents using MCP, here's a tool definition:

```json
{
  "name": "agentla_capture",
  "description": "Capture and decode digital signals using AgentLA logic analyzer",
  "inputSchema": {
    "type": "object",
    "properties": {
      "protocol": {
        "type": "string",
        "enum": ["I2C", "SPI", "UART", "PWM", "RAW"],
        "description": "Protocol to decode"
      },
      "channels": {
        "type": "array",
        "items": {"type": "integer"},
        "description": "GPIO pins to capture"
      },
      "sample_rate": {
        "type": "integer",
        "description": "Sample rate in Hz (100000-100000000)"
      },
      "samples": {
        "type": "integer",
        "description": "Number of samples to capture"
      },
      "protocol_params": {
        "type": "object",
        "description": "Protocol-specific parameters (sda, scl, cs, etc.)"
      }
    },
    "required": ["protocol", "channels", "sample_rate"]
  }
}
```

## Resources

- [AgentLA Protocol Specification](../openspec/changes/rp2350-logic-analyzer/specs/agent-protocol/spec.md)
- [Python SDK API Reference](../sdk/) (coming soon)
- [Example Captures](../test/) (test scripts and expected outputs)
- [gusmanb/logicanalyzer](https://github.com/gusmanb/logicanalyzer) - Original firmware source
