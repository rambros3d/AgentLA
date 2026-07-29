"""Protocol event data classes for AgentLA SDK."""

from dataclasses import dataclass
from typing import Optional


@dataclass
class I2CEvent:
    """I2C decoded event."""
    
    timestamp: float
    type: str  # START, STOP, ADDR, DATA, ACK, NACK
    value: Optional[int] = None  # Data byte or address
    rw: Optional[bool] = None  # True for Read, False for Write
    text: str = ""
    
    def __str__(self):
        if self.type == "ADDR":
            rw_str = "R" if self.rw else "W"
            return f"{self.timestamp:.6f} I2C ADDR 0x{self.value:02X} {rw_str}"
        elif self.type == "DATA":
            return f"{self.timestamp:.6f} I2C DATA 0x{self.value:02X}"
        else:
            return f"{self.timestamp:.6f} I2C {self.type}"


@dataclass
class SPIEvent:
    """SPI decoded event."""
    
    timestamp: float
    type: str  # CS_ASSERT, CS_DEASSERT, MOSI, MISO
    value: Optional[int] = None  # Data byte
    text: str = ""
    
    def __str__(self):
        if self.type in ("MOSI", "MISO"):
            return f"{self.timestamp:.6f} SPI {self.type} 0x{self.value:02X}"
        else:
            return f"{self.timestamp:.6f} SPI {self.type}"


@dataclass
class UARTEvent:
    """UART decoded event."""
    
    timestamp: float
    byte: Optional[int] = None  # Data byte value
    error: Optional[str] = None  # ERR_PARITY, ERR_FRAMING, or None
    text: str = ""
    
    def __str__(self):
        if self.error:
            return f"{self.timestamp:.6f} UART 0x{self.byte:02X} {self.error}"
        elif self.byte is not None:
            return f"{self.timestamp:.6f} UART 0x{self.byte:02X}"
        else:
            return f"{self.timestamp:.6f} UART BREAK"


@dataclass
class PWMResult:
    """PWM measurement result."""
    
    frequency: float = 0.0  # Hz
    period: float = 0.0  # seconds
    duty_cycle: float = 0.0  # percentage
    text: str = ""
    
    def __str__(self):
        if self.frequency > 0:
            return f"PWM FREQ {self.frequency:.1f} Hz PERIOD {self.period:.6f}s DUTY {self.duty_cycle:.1f}%"
        else:
            return "PWM NO_SIGNAL"


@dataclass
class ADCResult:
    """ADC sampling result."""
    
    pin: int = 0
    sample_rate: int = 100000
    samples: list = None
    text: str = ""
    
    def __post_init__(self):
        if self.samples is None:
            self.samples = []
    
    @property
    def num_samples(self) -> int:
        """Number of ADC samples collected."""
        return len(self.samples)
    
    @property
    def mean(self) -> float:
        """Mean ADC value (0-4095 for 12-bit)."""
        return sum(self.samples) / len(self.samples) if self.samples else 0.0
    
    @property
    def voltage(self) -> float:
        """Mean voltage (0-3.3V assuming 3.3V reference)."""
        return self.mean * 3.3 / 4095.0
    
    def __str__(self):
        if self.samples:
            return f"ADC pin={self.pin} rate={self.sample_rate} samples={self.num_samples} mean={self.mean:.1f} ({self.voltage:.3f}V)"
        else:
            return "ADC NO_DATA"
