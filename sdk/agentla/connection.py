"""Serial connection handler for AgentLA devices."""

import serial
import serial.tools.list_ports
import time
from .exceptions import DeviceNotFoundError, DeviceError, TimeoutError


class Connection:
    """Manages serial connection to AgentLA device."""
    
    # AgentLA USB identifiers
    VENDOR_ID = 0x1209
    PRODUCT_ID = 0x414C
    
    def __init__(self, port=None, baudrate=115200, timeout=2):
        """
        Initialize connection to AgentLA device.
        
        Args:
            port: Serial port path. If None, auto-detects AgentLA device.
            baudrate: Serial baud rate (default: 115200)
            timeout: Serial timeout in seconds (default: 2)
        """
        self.baudrate = baudrate
        self.timeout = timeout
        self._serial = None
        
        if port is None:
            port = self._find_device()
        
        self._serial = serial.Serial(
            port=port,
            baudrate=baudrate,
            timeout=timeout,
            write_timeout=timeout
        )
        
        # Wait for device to initialize
        time.sleep(0.5)
        self._serial.reset_input_buffer()
        self._serial.reset_output_buffer()
    
    def _find_device(self):
        """Auto-detect AgentLA device on available serial ports."""
        ports = serial.tools.list_ports.comports()
        
        for port in ports:
            if port.vid == self.VENDOR_ID and port.pid == self.PRODUCT_ID:
                return port.device
        
        # Try to find by description
        for port in ports:
            if 'agentla' in (port.description or '').lower():
                return port.device
        
        raise DeviceNotFoundError(
            "No AgentLA device found. Check USB connection."
        )
    
    def send_command(self, command):
        """
        Send command to device and return response.
        
        Args:
            command: Command string to send
        
        Returns:
            Response string from device
        """
        if not self._serial or not self._serial.is_open:
            raise DeviceError("Connection closed")
        
        # Send command
        self._serial.write(f"{command}\r\n".encode())
        
        # Skip echo line
        echo = self._read_line()
        
        # Read response
        response = self._read_line()
        return response
    
    def send_command_raw(self, command):
        """
        Send command without reading response.
        
        Args:
            command: Command string to send
        """
        if not self._serial or not self._serial.is_open:
            raise DeviceError("Connection closed")
        
        self._serial.write(f"{command}\r\n".encode())
    
    def read_response(self):
        """Read a single response line from device."""
        return self._read_line()
    
    def read_events(self, until_done=True):
        """
        Read decoded events until DONE or ERR.
        
        Args:
            until_done: If True, reads until DONE/ERR line
        
        Returns:
            List of event strings
        """
        events = []
        
        while True:
            line = self._read_line()
            
            if not line:
                continue
            
            if line.startswith('>'):
                continue  # Skip echo
            
            if line.startswith('DONE'):
                events.append(line)
                break
            
            if line.startswith('ERR'):
                events.append(line)
                break
            
            events.append(line)
            
            if not until_done:
                break
        
        return events
    
    def _read_line(self):
        """Read a single line from serial port."""
        try:
            data = self._serial.readline()
            if data:
                return data.decode('utf-8', errors='replace').strip()
            return ''
        except serial.SerialException as e:
            raise DeviceError(f"Serial error: {e}") from e
    
    def flush(self):
        """Flush serial buffers."""
        if self._serial and self._serial.is_open:
            self._serial.reset_input_buffer()
            self._serial.reset_output_buffer()
    
    def close(self):
        """Close serial connection."""
        if self._serial and self._serial.is_open:
            self._serial.close()
    
    def __enter__(self):
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()
        return False
    
    def __del__(self):
        self.close()
