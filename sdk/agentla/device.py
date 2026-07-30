"""Device class for AgentLA SDK."""

from dataclasses import dataclass
from enum import Enum
from typing import List, Optional
from .connection import Connection
from .capture import RawCapture
from .protocols import I2CEvent, SPIEvent, UARTEvent, PWMResult, ADCResult
from .adapter import I2CDevice, UARTDevice
from .exceptions import DeviceError


class Status(Enum):
    """Device status states."""
    IDLE = "IDLE"
    CAPTURING = "CAPTURING"
    DECODING = "DECODING"


@dataclass
class DeviceInfo:
    """Device identification information."""
    model: str
    version: str
    mcu: str
    role: str = "ANALYZER"
    channels: int = 24
    max_sample_rate: int = 100_000_000
    blast_freq: int = 200_000_000
    buffer_size: int = 393216

    @property
    def is_adapter(self) -> bool:
        """True if this device exposes I2C/UART adapter mode."""
        return self.role == "ADAPTER"


class Device:
    """AgentLA device interface."""
    
    def __init__(self, connection: Connection):
        """
        Initialize device with serial connection.
        
        Args:
            connection: Serial connection to device
        """
        self._conn = connection
        self._channels = None
        self._sample_rate = None
    
    def identify(self) -> DeviceInfo:
        """Identify device and return capabilities."""
        response = self._conn.send_command("IDENTIFY")
        parts = response.split()
        model = parts[0] if len(parts) > 0 else "AGENTLA"
        version = parts[1] if len(parts) > 1 else "v1.0"
        mcu = parts[2] if len(parts) > 2 else "RP2350"
        role = "ANALYZER"
        for p in parts[1:]:
            if "ADAPTER" in p.upper():
                role = "ADAPTER"
                break
        channels = 24
        max_rate = 100_000_000
        blast_freq = 200_000_000
        buffer_size = 393216
        for _ in range(4):
            line = self._conn.read_response()
            if line.startswith("FREQ:"):
                max_rate = int(line.split(":")[1])
            elif line.startswith("BLASTFREQ:"):
                blast_freq = int(line.split(":")[1])
            elif line.startswith("BUFFER:"):
                buffer_size = int(line.split(":")[1])
            elif line.startswith("CHANNELS:"):
                channels = int(line.split(":")[1])
        return DeviceInfo(
            model=model, version=version, mcu=mcu, role=role,
            channels=channels, max_sample_rate=max_rate,
            blast_freq=blast_freq, buffer_size=buffer_size,
        )
    
    def configure(
        self,
        channels: List[int],
        sample_rate: int = 1_000_000,
        trigger: Optional[str] = None,
        trigger_pin: Optional[int] = None,
        trigger_pattern: Optional[int] = None
    ):
        """
        Configure device channels and sample rate.
        
        Args:
            channels: List of GPIO pin numbers to capture
            sample_rate: Sample rate in Hz (100000-100000000)
            trigger: Trigger type: 'RISING', 'FALLING', or 'PATTERN'
            trigger_pin: Pin number for edge trigger
            trigger_pattern: Bit pattern for pattern trigger
        """
        # Build CONFIGURE command
        ch_str = ",".join(str(c) for c in channels)
        cmd = f"CONFIGURE channels={ch_str} rate={sample_rate}"
        
        if trigger:
            cmd += f" trigger={trigger}"
        if trigger_pin is not None:
            cmd += f" pin={trigger_pin}"
        if trigger_pattern is not None:
            cmd += f" pattern={trigger_pattern}"
        
        response = self._conn.send_command(cmd)
        
        if response.startswith("ERR"):
            raise DeviceError(response)
        
        self._channels = channels
        self._sample_rate = sample_rate
    
    def capture_raw(
        self,
        samples: int = 4096,
        trigger: Optional[dict] = None,
        pretrigger: int = 50
    ) -> RawCapture:
        """
        Capture raw pin states.
        
        Args:
            samples: Number of samples to capture
            trigger: Trigger config: {'pin': int, 'edge': 'rising'|'falling'}
            pretrigger: Pre-trigger percentage (0-100)
        
        Returns:
            RawCapture with captured data
        """
        cmd = f"CAPTURE samples={samples} mode=RAW"
        
        if pretrigger != 50:
            cmd += f" pretrigger={pretrigger}"
        
        if trigger:
            if 'edge' in trigger:
                edge = trigger['edge'].upper()
                pin = trigger.get('pin', 0)
                # Configure trigger first
                self._conn.send_command(f"CONFIGURE trigger={edge} pin={pin}")
        
        # Flush any stale data
        self._conn.flush()
        
        # Send capture command
        self._conn.send_command_raw(cmd)
        
        # Read CAPTURING response
        response = self._conn.read_response()
        
        # Read RAW data lines
        raw_data = []
        while True:
            line = self._conn.read_response()
            
            if not line:
                continue
            
            if line.startswith("RAW "):
                hex_val = line.split()[1]
                raw_data.append(int(hex_val, 16))
            elif line.startswith("DONE"):
                # Parse DONE response
                break
            elif line.startswith("ERR"):
                raise DeviceError(line)
        
        # Build raw capture object
        data = bytes(raw_data)
        return RawCapture(
            data=data,
            sample_rate=self._sample_rate,
            num_channels=len(self._channels) if self._channels else 8,
            duration=samples / self._sample_rate if self._sample_rate else 0
        )
    
    def capture_decoded(
        self,
        protocol: str,
        samples: int = 4096,
        sda: int = 0,
        scl: int = 1,
        cs: int = 0,
        sclk: int = 1,
        mosi: int = 2,
        miso: int = 3,
        rx: int = 0,
        baud: int = 115200,
        pretrigger: int = 50
    ):
        """
        Capture and decode protocol data.
        
        Args:
            protocol: Protocol to decode: 'I2C', 'SPI', 'UART', or 'PWM'
            samples: Number of samples to capture
            sda: SDA pin for I2C
            scl: SCL pin for I2C
            cs: CS pin for SPI
            sclk: SCLK pin for SPI
            mosi: MOSI pin for SPI
            miso: MISO pin for SPI
            rx: RX pin for UART
            baud: Baud rate for UART
            pretrigger: Pre-trigger percentage (0-100)
        
        Returns:
            List of protocol events
        """
        cmd = f"CAPTURE samples={samples} mode=DECODED protocol={protocol}"
        
        # Add protocol-specific parameters
        if protocol == "I2C":
            cmd += f" sda={sda} scl={scl}"
        elif protocol == "SPI":
            cmd += f" cs={cs} sclk={sclk} mosi={mosi} miso={miso}"
        elif protocol == "UART":
            cmd += f" rx={rx} baud={baud}"
        elif protocol == "PWM":
            cmd += f" sda={sda}"
        
        if pretrigger != 50:
            cmd += f" pretrigger={pretrigger}"
        
        # Send capture command
        self._conn.send_command_raw(cmd)
        
        # Read CAPTURING response
        response = self._conn.read_response()
        
        # Read decoded events
        events = []
        while True:
            line = self._conn.read_response()
            
            if not line:
                continue
            
            if line.startswith('>'):
                continue
            
            if line.startswith('DONE'):
                break
            
            if line.startswith('ERR'):
                raise DeviceError(line)
            
            # Parse event based on protocol
            event = self._parse_event(line, protocol)
            if event:
                events.append(event)
        
        return events
    
    def _parse_event(self, line: str, protocol: str):
        """Parse a decoded event line into appropriate event object."""
        parts = line.split()
        
        if len(parts) < 2:
            return None
        
        timestamp = float(parts[0])
        proto = parts[1]
        event_type = parts[2] if len(parts) > 2 else ""
        
        if protocol == "I2C":
            value = int(parts[3], 16) if len(parts) > 3 and parts[3].startswith("0x") else None
            rw = parts[4] if len(parts) > 4 else None
            return I2CEvent(
                timestamp=timestamp,
                type=event_type,
                value=value,
                rw=rw == "R" if rw else None,
                text=line
            )
        elif protocol == "SPI":
            value = int(parts[3], 16) if len(parts) > 3 and parts[3].startswith("0x") else None
            return SPIEvent(
                timestamp=timestamp,
                type=event_type,
                value=value,
                text=line
            )
        elif protocol == "UART":
            byte_val = int(parts[3], 16) if len(parts) > 3 and parts[3].startswith("0x") else None
            error = parts[4] if len(parts) > 4 else None
            return UARTEvent(
                timestamp=timestamp,
                byte=byte_val,
                error=error,
                text=line
            )
        elif protocol == "PWM":
            # TODO: Parse actual PWM values from response text
            # For now, return a result with the raw text
            return PWMResult(
                frequency=0.0,
                period=0.0,
                duty_cycle=0.0,
                text=line
            )
        
        return None
    
    def measure_pwm(self, pin: int = 0) -> PWMResult:
        """
        Measure PWM signal properties.
        
        Args:
            pin: GPIO pin to measure
        
        Returns:
            PWMResult with frequency, period, and duty cycle
        """
        cmd = f"MEASURE PWM pin={pin}"
        response = self._conn.send_command(cmd)
        
        # Parse response: "PWM FREQ 1000.0 Hz PERIOD 0.001s DUTY 50.0%"
        parts = response.split()
        freq = float(parts[2]) if "FREQ" in parts else 0.0
        period = float(parts[5].rstrip('s')) if "PERIOD" in parts else 0.0
        duty = float(parts[7].rstrip('%')) if "DUTY" in parts else 0.0
        
        return PWMResult(
            frequency=freq,
            period=period,
            duty_cycle=duty,
            text=response
        )
    
    def adc_sample(
        self,
        pin: int = 26,
        sample_rate: int = 100_000,
        samples: int = 1024
    ) -> ADCResult:
        """
        Sample analog voltage on ADC-capable pins (26-29).
        
        Args:
            pin: GPIO pin number (must be 26-29 for ADC)
            sample_rate: Sample rate in Hz (max 500000)
            samples: Number of samples to collect (max 32768)
        
        Returns:
            ADCResult with sampled data
        
        Raises:
            DeviceError: If pin is invalid or capture fails
        """
        # Validate pin
        if pin < 26 or pin > 29:
            raise DeviceError(f"ERR ADC pin must be 26-29, got {pin}")
        
        # Clamp values to firmware limits
        if sample_rate > 500_000:
            sample_rate = 500_000
        if samples > 32768:
            samples = 32768
        
        cmd = f"ADC pin={pin} rate={sample_rate} samples={samples}"
        
        # Flush any stale data
        self._conn.flush()
        
        # Send command
        self._conn.send_command_raw(cmd)
        
        # Read CAPTURING response
        response = self._conn.read_response()
        
        if response.startswith("ERR"):
            raise DeviceError(response)
        
        # Read ADC data lines
        adc_values = []
        while True:
            line = self._conn.read_response()
            
            if not line:
                continue
            
            if line.startswith('>'):
                continue
            
            if line.startswith("ADC DONE"):
                break
            
            if line.startswith("ERR"):
                raise DeviceError(line)
            
            if line.startswith("ADC "):
                try:
                    value = int(line.split()[1])
                    adc_values.append(value)
                except (IndexError, ValueError):
                    continue
        
        return ADCResult(
            pin=pin,
            sample_rate=sample_rate,
            samples=adc_values,
            text=response
        )
    
    def status(self) -> Status:
        """
        Get current device status.
        
        Returns:
            Status enum value
        """
        response = self._conn.send_command("STATUS")
        
        if "IDLE" in response:
            return Status.IDLE
        elif "CAPTURING" in response:
            return Status.CAPTURING
        elif "DECODING" in response:
            return Status.DECODING
        
        return Status.IDLE
    
    def adapter(self):
        """Return an I2CDevice and UARTDevice for the adapter CDC port.

        Returns:
            Tuple of (I2CDevice, UARTDevice) connected to the adapter CDC
        """
        return I2CDevice(self._conn), UARTDevice(self._conn)

    def close(self):
        """Close device connection."""
        self._conn.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False
