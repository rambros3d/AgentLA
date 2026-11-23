"""
AgentLA Native USB-to-SPI FTDI MPSSE Adapter Client.

Enables direct communication with AgentLA's hardware SPI master using
pyftdi or raw libusb without going through the HTTP REST API.
"""

from typing import Optional

try:
    from pyftdi.spi import SpiController, SpiPort
    HAS_PYFTDI = True
except ImportError:
    HAS_PYFTDI = False

class AgentLaSpi:
    """AgentLA Native FTDI MPSSE SPI Master Interface."""

    def __init__(self, vid: int = 0x1209, pid: int = 0x414D, interface_index: int = 1):
        self.vid = vid
        self.pid = pid
        self.interface_index = interface_index
        self.controller: Optional[SpiController] = None
        self.port: Optional[SpiPort] = None

    def connect(self, cs: int = 0, freq_hz: float = 1_000_000.0, mode: int = 0) -> bool:
        """Connect to AgentLA SPI MPSSE interface via pyftdi URL."""
        if not HAS_PYFTDI:
            raise ImportError("pyftdi is required. Install via 'pip install pyftdi'.")

        url = f"ftdi://{self.vid:#06x}:{self.pid:#06x}:{self.interface_index}"
        self.controller = SpiController(cs_count=1)
        self.controller.configure(url)
        self.port = self.controller.get_port(cs=cs, freq=freq_hz, mode=mode)
        return self.port is not None

    def transfer(self, tx_data: bytes, read_len: int = 0) -> bytes:
        """Perform full-duplex or half-duplex SPI transfer."""
        if self.port is None:
            raise ConnectionError("AgentLaSpi is not connected.")
        return bytes(self.port.exchange(tx_data, readlen=read_len))

    def write(self, tx_data: bytes):
        """Write bytes to SPI MOSI."""
        if self.port is None:
            raise ConnectionError("AgentLaSpi is not connected.")
        self.port.write(tx_data)

    def read(self, length: int) -> bytes:
        """Read bytes from SPI MISO."""
        if self.port is None:
            raise ConnectionError("AgentLaSpi is not connected.")
        return bytes(self.port.read(length))

    def close(self):
        """Close SPI connection."""
        if self.controller:
            self.controller.terminate()
            self.controller = None
            self.port = None
