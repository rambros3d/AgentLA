"""Adapter device classes for AgentLA SDK - I2C and UART active bus control."""

from .connection import Connection
from .exceptions import DeviceError, TimeoutError


class I2CDevice:
    """I2C master adapter for AgentLA.

    Provides register-level read/write access to I2C slave devices
    connected to the AgentLA adapter CDC interface.

    Args:
        connection: Connection instance connected to the adapter CDC port
        addr: Default I2C slave address for subsequent operations
    """

    def __init__(self, connection: Connection, addr: int = 0x00, claim_handle: int = None):
        self._conn = connection
        self._addr = addr
        self._claim_handle = claim_handle

    def claim(self, pins, function=2, observer: bool = False) -> int:
        """Claim flexible I2C pins and store the resulting handle."""
        pin_str = ",".join(str(p) for p in pins)
        cmd = f"PINCLAIM pins={pin_str} func={int(function)} observer={1 if observer else 0}"
        response = self._conn.send_command(cmd)
        if response.startswith("ERR"):
            raise DeviceError(response)
        for part in response.split():
            if part.startswith("HANDLE="):
                self._claim_handle = int(part.split("=", 1)[1])
                return self._claim_handle
        raise DeviceError(f"Unexpected claim response: {response}")

    @property
    def address(self) -> int:
        """Default I2C slave address."""
        return self._addr

    def read(self, addr: int = None, reg: int = 0x00, length: int = 1,
             timeout_ms: int = 100, claim_handle: int = None) -> bytes:
        """Read bytes from an I2C slave register.

        Args:
            addr: I2C slave address (7-bit). Uses self._addr if omitted.
            reg: Register pointer to start reading from.
            length: Number of bytes to read (max 64).
            timeout_ms: Timeout for the I2C transaction in milliseconds.

        Returns:
            Bytes read from the I2C slave.

        Raises:
            DeviceError: If the I2C NACKs or the bus is stuck.
            TimeoutError: If the transaction times out.
        """
        target_addr = addr if addr is not None else self._addr
        handle = self._claim_handle if claim_handle is None else claim_handle
        if handle is None:
            from .exceptions import DeviceError
            raise DeviceError("ERR I2C requires ownership handle")
        cmd = f"handle={handle} I2C_READ addr=0x{target_addr:02X} reg=0x{reg:02X} len={length} timeout={timeout_ms}"
        response = self._conn.send_command(cmd)

        if response.startswith("ERR"):
            raise DeviceError(response)

        if not response.startswith("I2C DATA "):
            raise DeviceError(f"Unexpected I2C_READ response: {response}")

        hex_data = response[len("I2C DATA "):]
        if not hex_data.strip():
            return b""
        byte_values = [int(b, 16) for b in hex_data.split(",")]
        return bytes(byte_values)

    def write(self, addr: int = None, reg: int = 0x00, data: bytes = b"",
              timeout_ms: int = 100, claim_handle: int = None) -> int:
        """Write bytes to an I2C slave register.

        Args:
            addr: I2C slave address (7-bit). Uses self._addr if omitted.
            reg: Register pointer to write to.
            data: Bytes to write (max 64).
            timeout_ms: Timeout for the I2C transaction in milliseconds.

        Returns:
            Number of bytes written.

        Raises:
            DeviceError: If the I2C NACKs or the bus is stuck.
            TimeoutError: If the transaction times out.
        """
        target_addr = addr if addr is not None else self._addr
        handle = self._claim_handle if claim_handle is None else claim_handle
        if handle is None:
            from .exceptions import DeviceError
            raise DeviceError("ERR I2C requires ownership handle")
        data_hex = ",".join(f"{b:02X}" for b in data)
        cmd = f"I2C_WRITE handle={handle} addr=0x{target_addr:02X} reg=0x{reg:02X} data={data_hex} timeout={timeout_ms}"
        response = self._conn.send_command(cmd)

        if response.startswith("ERR"):
            raise DeviceError(response)

        if not response.startswith("OK I2C WRITE"):
            raise DeviceError(f"Unexpected I2C_WRITE response: {response}")

        return len(data)

    def clear_bus(self, claim_handle: int = None) -> bool:
        """Recover a stuck I2C bus by toggling SCL 9 times.

        Returns:
            True if the bus clear command was accepted.
        """
        handle = self._claim_handle if claim_handle is None else claim_handle
        if handle is None:
            from .exceptions import DeviceError
            raise DeviceError("ERR I2C requires ownership handle")
        cmd = f"I2C_CLEAR handle={handle}"
        response = self._conn.send_command(cmd)
        if response.startswith("ERR"):
            raise DeviceError(response)
        return "OK I2C BUS CLEARED" in response


class UARTDevice:
    """UART master adapter for AgentLA.

    Provides transmit and receive access to UART serial devices
    connected to the AgentLA adapter CDC interface.

    Args:
        connection: Connection instance connected to the adapter CDC port
    """

    def __init__(self, connection: Connection, claim_handle: int = None):
        self._conn = connection
        self._claim_handle = claim_handle

    def claim(self, pins, function=1, observer: bool = False) -> int:
        """Claim flexible UART pins and store the resulting handle."""
        from .device import Device
        _ = Device
        pin_str = ",".join(str(p) for p in pins)
        cmd = f"PINCLAIM pins={pin_str} func={int(function)} observer={1 if observer else 0}"
        response = self._conn.send_command(cmd)
        if response.startswith("ERR"):
            from .exceptions import DeviceError
            raise DeviceError(response)
        for part in response.split():
            if part.startswith("HANDLE="):
                self._claim_handle = int(part.split("=", 1)[1])
                return self._claim_handle
        from .exceptions import DeviceError
        raise DeviceError(f"Unexpected claim response: {response}")

    def write(self, data: bytes, baud: int = 115200, claim_handle: int = None) -> int:
        """Write bytes to a UART serial device.

        Args:
            data: Bytes to transmit.
            baud: Baud rate (must be between 1200 and 921600).

        Returns:
            Number of bytes written.

        Raises:
            DeviceError: If the baud rate is invalid or the write fails.
        """
        if baud < 1200 or baud > 921600:
            raise DeviceError(
                f"ERR invalid baud rate, min=1200 max=921600 (got {baud})"
            )

        handle = self._claim_handle if claim_handle is None else claim_handle
        if handle is None:
            from .exceptions import DeviceError
            raise DeviceError("ERR UART requires ownership handle")
        data_hex = ",".join(f"{b:02X}" for b in data)
        cmd = f"UART_WRITE handle={handle} baud={baud} data={data_hex}"
        response = self._conn.send_command(cmd)

        if response.startswith("ERR"):
            raise DeviceError(response)

        if not response.startswith("OK UART WROTE"):
            raise DeviceError(f"Unexpected UART_WRITE response: {response}")

        return len(data)

    def read(self, max_len: int = 64, baud: int = 115200,
             timeout_ms: int = 1000, claim_handle: int = None) -> bytes:
        """Read up to max_len bytes from a UART serial device.

        Args:
            max_len: Maximum number of bytes to read (max 256).
            baud: Baud rate (must be between 1200 and 921600).
            timeout_ms: Read timeout in milliseconds.

        Returns:
            Bytes received. May be empty if no data arrived within timeout.
        """
        if max_len < 1:
            max_len = 1
        if max_len > 256:
            max_len = 256

        handle = self._claim_handle if claim_handle is None else claim_handle
        if handle is None:
            from .exceptions import DeviceError
            raise DeviceError("ERR UART requires ownership handle")
        cmd = f"UART_READ handle={handle} baud={baud} len={max_len} timeout={timeout_ms}"
        response = self._conn.send_command(cmd)

        if response.startswith("ERR"):
            raise DeviceError(response)

        if not response.startswith("UART DATA"):
            raise DeviceError(f"Unexpected UART_READ response: {response}")

        hex_data = response[len("UART DATA "):]
        if not hex_data.strip():
            return b""
        byte_values = [int(b, 16) for b in hex_data.split(",")]
        return bytes(byte_values)

    def send(self, data: bytes, baud: int = 115200) -> bytes:
        """Send data and optionally read a response.

        Convenience method that writes data then reads a reply.

        Args:
            data: Bytes to transmit.
            baud: Baud rate.

        Returns:
            The bytes received as a response (may be empty).
        """
        self.write(data, baud=baud)
        return self.read(max_len=64, baud=baud, timeout_ms=1000)