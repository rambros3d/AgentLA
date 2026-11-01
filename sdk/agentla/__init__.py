"""
AgentLA Python SDK - Logic Analyzer for LLM Agents

This SDK provides a programmatic interface to AgentLA logic analyzer
devices for automated electronics debugging.
"""

from .connection import Connection
from .device import Device, DeviceInfo, Status
from .capture import RawCapture, CapturedEvent, unpack_rle_packets, export_vcd
from .protocols import I2CEvent, SPIEvent, UARTEvent, PWMResult, ADCResult
from .adapter import I2CDevice, UARTDevice
from .sigrok import export_sigrok, SigrokDecoder, SigrokDecoderError
from .exceptions import DeviceNotFoundError, DeviceError, TimeoutError


def connect(port=None, baudrate=115200, timeout=2):
    """
    Connect to an AgentLA device.

    Args:
        port: Serial port path (e.g., '/dev/ttyACM0'). If None, auto-detects.
        baudrate: Serial baud rate (default: 115200)
        timeout: Connection timeout in seconds (default: 2)

    Returns:
        Device object context manager

    Raises:
        DeviceNotFoundError: If no AgentLA device is found

    Example:
        with agentla.connect() as device:
            info = device.identify()
            print(f"Found: {info.model}")
    """
    conn = Connection(port=port, baudrate=baudrate, timeout=timeout)
    return Device(conn)


__version__ = "0.1.0"
__all__ = [
    "connect",
    "Device",
    "DeviceInfo",
    "Status",
    "RawCapture",
    "unpack_rle_packets",
    "export_vcd",
    "CapturedEvent",
    "I2CEvent",
    "SPIEvent",
    "UARTEvent",
    "PWMResult",
    "ADCResult",
    "I2CDevice",
    "UARTDevice",
    "SigrokDecoder",
    "SigrokDecoderError",
    "export_sigrok",
    "DeviceNotFoundError",
    "DeviceError",
    "TimeoutError",
]
