# AgentLA Python SDK

Python SDK for AgentLA Logic Analyzer - Designed for LLM Agent Integration.

## Overview

AgentLA is an RP2350-based logic analyzer that performs on-device protocol decode and outputs structured text that language models can directly parse and reason about. This SDK provides a clean Python interface for programmatic access.

## Installation

Install from source (package not yet published to PyPI):

```bash
git clone https://github.com/AgentLA/AgentLA
cd AgentLA/sdk
pip install -e .
```

## Quick Start

```python
import agentla

# Auto-detect and connect
with agentla.connect() as device:
    # Identify device
    info = device.identify()
    print(f"Found: {info.model} {info.channels}ch @ {info.max_sample_rate/1e6:.0f}MHz")
    
    # Configure for I2C analysis
    device.configure(
        channels=[0, 1],
        sample_rate=1_000_000,  # 1MHz for I2C
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
        print(f"{event.timestamp:.6f} {event.type}: {event}")
```

## Features

- **Auto-detection**: Automatically finds connected AgentLA devices
- **Protocol decode**: I2C, SPI, UART, PWM on-device decode
- **Raw capture**: Hex-encoded pin states for Sigrok/PulseView compatibility
- **Error handling**: Structured exceptions for device errors
- **Context manager**: Automatic connection cleanup

## API Reference

### `agentla.connect(port=None, baudrate=115200, timeout=2)`

Connect to an AgentLA device.

- `port`: Serial port path (e.g., `/dev/ttyACM0`). If None, auto-detects.
- `baudrate`: Serial baud rate (default: 115200)
- `timeout`: Connection timeout in seconds (default: 2)

Returns: `Device` object (context manager)

### `Device.identify()`

Identify device capabilities.

Returns: `DeviceInfo` with model, version, mcu, channels, max_sample_rate, blast_freq, buffer_size

### `Device.configure(channels, sample_rate, trigger=None, trigger_pin=None, trigger_pattern=None)`

Configure capture channels and sample rate.

- `channels`: List of GPIO pin numbers to capture
- `sample_rate`: Sample rate in Hz (100000-100000000)
- `trigger`: Trigger type: 'RISING', 'FALLING', or 'PATTERN'
- `trigger_pin`: Pin number for edge trigger
- `trigger_pattern`: Bit pattern for pattern trigger

### `Device.capture_raw(samples=4096, trigger=None, pretrigger=50)`

Capture raw pin states.

Returns: `RawCapture` with data, sample_rate, num_channels, duration

### `Device.capture_decoded(protocol, samples=4096, ...)`

Capture and decode protocol data.

- `protocol`: Protocol to decode: 'I2C', 'SPI', 'UART', or 'PWM'
- Protocol-specific parameters: sda, scl (I2C), cs, sclk, mosi, miso (SPI), rx, baud (UART)

Returns: List of protocol events

### `Device.measure_pwm(pin=0)`

Measure PWM signal properties.

Returns: `PWMResult` with frequency, period, duty_cycle

### `Device.status()`

Get current device status.

Returns: `Status.IDLE`, `Status.CAPTURING`, or `Status.DECODING`

## Protocol Events

### I2CEvent

```python
@dataclass
class I2CEvent:
    timestamp: float      # Timestamp in seconds
    type: str             # START, STOP, ADDR, DATA, ACK, NACK
    value: Optional[int]  # Data byte or address (for ADDR/DATA)
    rw: Optional[bool]    # True for Read, False for Write (for ADDR)
    text: str             # Raw event text from device
```

### SPIEvent

```python
@dataclass
class SPIEvent:
    timestamp: float      # Timestamp in seconds
    type: str             # CS_ASSERT, CS_DEASSERT, MOSI, MISO
    value: Optional[int]  # Data byte (for MOSI/MISO)
    text: str             # Raw event text from device
```

### UARTEvent

```python
@dataclass
class UARTEvent:
    timestamp: float      # Timestamp in seconds
    byte: Optional[int]   # Data byte value
    error: Optional[str]  # ERR_PARITY, ERR_FRAMING, or None
    text: str             # Raw event text from device
```

### PWMResult

```python
@dataclass
class PWMResult:
    frequency: float      # Frequency in Hz
    period: float         # Period in seconds
    duty_cycle: float     # Duty cycle in percentage
    text: str             # Raw result text from device
```

## Error Handling

```python
from agentla import DeviceError, DeviceNotFoundError

try:
    with agentla.connect() as device:
        device.configure(channels=[0], sample_rate=100_000_000)  # Too high
except DeviceError as e:
    print(f"Device error: {e}")
except DeviceNotFoundError:
    print("No AgentLA device found")
```

## Examples

### I2C EEPROM Read

```python
import agentla

with agentla.connect() as device:
    device.configure(channels=[0, 1], sample_rate=1_000_000)
    events = device.capture_decoded(
        protocol='I2C', sda=0, scl=1, samples=8192
    )
    
    for event in events:
        if event.type == 'ADDR':
            print(f"Address: 0x{event.value:02X} ({'R' if event.rw else 'W'})")
        elif event.type == 'DATA':
            print(f"Data: 0x{event.value:02X}")
```

### SPI Flash ID

```python
import agentla

with agentla.connect() as device:
    device.configure(channels=[0, 1, 2, 3], sample_rate=10_000_000)
    events = device.capture_decoded(
        protocol='SPI', cs=0, sclk=1, mosi=2, miso=3, samples=2048
    )
    
    for event in events:
        if event.type in ('MOSI', 'MISO'):
            print(f"{event.type}: 0x{event.value:02X}")
```

### Raw Capture for Sigrok

```python
import agentla

with agentla.connect() as device:
    device.configure(channels=[0, 1, 2, 3], sample_rate=10_000_000)
    capture = device.capture_raw(samples=4096)
    
    # Save for PulseView
    capture.export_sigrok('capture.sr')
```

## Requirements

- Python 3.8+
- pyserial 3.5+

## License

GPL-3.0 - See [LICENSE](../LICENSE) for details.

## Links

- [GitHub Repository](https://github.com/AgentLA/AgentLA)
- [Documentation](../docs/)
- [Firmware Source](../firmware/)
